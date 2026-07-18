pub mod controller;

use crate::{
    cursor_capture_server::SharedFrame, detectors::{self}, fps_handler::FpsHandler, settings::Settings, ui::UiSended,
};

use image::{DynamicImage, GrayImage};
use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

use std::{
    fmt::Display,
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, Sender},
    },
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, AsRefStr, Clone)]
pub enum BotState {
    Idle,
    LiquidLevelCalibration(Instant),
    MovemapThresholdCalibration(Instant),
    WaitingForBite(Instant),
    Cast,
    Catch,
    CastingCooldown(Instant),
    CatchingCooldown(Instant),
}

impl Display for BotState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            BotState::Idle => "Idle",
            BotState::LiquidLevelCalibration(..) => "LiquidLevelCalibration",
            BotState::MovemapThresholdCalibration(..) => "ThresholdCalibration",
            BotState::WaitingForBite(..) => "WaitingForBite",
            BotState::Cast => "Cast",
            BotState::Catch => "Catch",
            BotState::CastingCooldown(..) => "CastingCooldown",
            BotState::CatchingCooldown(..) => "CatchingCooldown",
        };
        write!(f, "{}", name)
    }
}

pub enum BotCommand {
    Start,
    Stop,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum DetectionMethod {
    Movemap,
    Audio,
    Ocr,
}

pub struct Bot {
    ocr_detector: detectors::ocr::Ocr,
    audio_detector: Option<detectors::onnx_audio::ONNXAudio>,
    movemap_detector: detectors::movemap::Movemap,
    shared_frame: SharedFrame,
    force_draw_frame: SharedFrame,
    state: BotState,
    settings: Arc<RwLock<Settings>>,
    rx: Receiver<BotCommand>,
    tx: Sender<UiSended>,
    controller: controller::Controller,
    last_frame_start: Instant,
    fps_handler: FpsHandler,
    egui_ctx: eframe::egui::Context,
}

unsafe impl Send for Bot {}

impl Bot {
    pub fn new(
        rx: Receiver<BotCommand>,
        tx: Sender<UiSended>,
        shared_frame: SharedFrame,
        force_draw_frame: SharedFrame,
        settings: Arc<RwLock<Settings>>,
        ocr_detector: detectors::ocr::Ocr,
        audio_detector: Option<detectors::onnx_audio::ONNXAudio>,
        movemap_detector: detectors::movemap::Movemap,
        egui_ctx: eframe::egui::Context,
    ) -> Bot {
        Bot {
            rx,
            tx,
            shared_frame,
            force_draw_frame,
            state: BotState::Idle,
            settings: Arc::clone(&settings),
            controller: controller::Controller::new(),
            ocr_detector,
            movemap_detector,
            audio_detector,
            egui_ctx,
            last_frame_start: Instant::now(),
            fps_handler: FpsHandler::default(),
        }
    }

    pub fn run(&mut self) {
        loop {
            self.last_frame_start = Instant::now();

            self.handle_commands();
            self.update();
            let detection_method = self.settings.read().unwrap().bot.detection_method.clone();
            let capture_fps = self.settings.read().unwrap().capture.fps;

            if detection_method == DetectionMethod::Audio {
                self.manage_sleep(30);
            } else {
                self.manage_sleep(capture_fps);
            }

            self.fps_handler.add(self.last_frame_start);

            if let Some(fps) = self.fps_handler.get_avg_fps() {
                let _ = self.tx.send(UiSended::SetBotFps(fps));
            }
        }
    }

    pub fn start(&mut self) {
        info!("===== Bot start =====");
        self.movemap_detector.reset();
        if self.settings.read().unwrap().bot.detection_method == DetectionMethod::Movemap {
            self.set_state(BotState::LiquidLevelCalibration(Instant::now()));
        } else {
            self.set_state(BotState::Cast);
        }
    }

    pub fn stop(&mut self) {
        info!("===== Bot stop =====");
        self.set_state(BotState::Idle);
        *self.force_draw_frame.lock().unwrap() = None;
        let _ = self.tx.send(UiSended::LiquidGap(None));
    }

    fn update(&mut self) {
        match self.state {
            BotState::Idle => {}
            BotState::LiquidLevelCalibration(time) => {
                let Some(difference_mask) = &self.get_difference_mask() else {
                    return;
                };

                self.movemap_detector
                    .liquid_level_calibrator
                    .add(difference_mask);

                if time.elapsed()
                    > Duration::from_millis(
                        self.settings
                            .read()
                            .unwrap()
                            .movemap
                            .liquid_detection_delay_ms,
                    )
                {
                    if !self.movemap_detector.set_liquid_level() {
                        info!("Liquid Calibration Result: Retry");
                        self.set_state(BotState::LiquidLevelCalibration(Instant::now()));
                        return;
                    }

                    info!(
                        "Liquid Calibration Result: {}",
                        self.movemap_detector.liquid_level.unwrap()
                    );

                    let detection_gap = self.movemap_detector.get_liquid_gap().unwrap();

                    let _ = self.tx.send(UiSended::LiquidGap(Some(detection_gap)));
                    self.set_state(BotState::Cast);
                }
            }
            BotState::MovemapThresholdCalibration(time) => {
                let Some(difference_mask) = self.get_difference_mask() else {
                    return;
                };

                self.movemap_detector.threshold_calibrator.add(
                    &difference_mask,
                    self.movemap_detector.get_liquid_gap().unwrap(),
                );

                if time.elapsed()
                    > Duration::from_millis(
                        self.settings.read().unwrap().movemap.calibration_time_ms,
                    )
                {
                    if !self.movemap_detector.set_threshold() {
                        info!("Threshold Calibration Result: Retry");
                        self.set_state(BotState::MovemapThresholdCalibration(Instant::now()));
                        return;
                    }

                    info!(
                        "Threshold Calibration Result: {}",
                        self.movemap_detector.threshold.unwrap()
                    );
                    self.set_state(BotState::WaitingForBite(Instant::now()));
                }
            }
            BotState::Cast => {
                self.controller.cast();
                self.set_state(BotState::CastingCooldown(Instant::now()));
            }
            BotState::Catch => {
                self.controller.catch();
                self.set_state(BotState::CatchingCooldown(Instant::now()));
            }
            BotState::CastingCooldown(time) => {
                if self.settings.read().unwrap().bot.detection_method == DetectionMethod::Movemap {
                    self.get_difference_mask();
                }

                if time.elapsed()
                    > Duration::from_millis(self.settings.read().unwrap().bot.casting_delay_millis)
                {
                    if self.settings.read().unwrap().bot.use_potions {
                        self.controller.use_potions();
                    }

                    if self.settings.read().unwrap().bot.detection_method
                        == DetectionMethod::Movemap
                        && self.movemap_detector.threshold.is_none()
                    {
                        self.set_state(BotState::MovemapThresholdCalibration(Instant::now()));
                    } else {
                        self.set_state(BotState::WaitingForBite(Instant::now()));
                    }
                }
            }
            BotState::CatchingCooldown(time) => {
                if self.settings.read().unwrap().bot.detection_method == DetectionMethod::Movemap {
                    self.get_difference_mask();
                }

                if time.elapsed()
                    > Duration::from_millis(self.settings.read().unwrap().bot.catching_delay_millis)
                {
                    self.set_state(BotState::Cast);
                }
            }
            BotState::WaitingForBite(time) => {
                if time.elapsed() > Duration::from_secs(30) {
                    info!("Too long ({}s), recast.", time.elapsed().as_secs());
                    self.set_state(BotState::Catch);
                }

                let detection_method = self.settings.read().unwrap().bot.detection_method.clone();

                match detection_method {
                    DetectionMethod::Movemap => {
                        let Some(difference_mask) = self.get_difference_mask() else {
                            return;
                        };

                        if self.movemap_detector.is_detected(&difference_mask) {
                            self.set_state(BotState::Catch);
                        }
                    }
                    DetectionMethod::Ocr => {
                        let rgb_frame = {
                            let guard = self.shared_frame.lock().unwrap();
                            let Some(ref frame) = *guard else {
                                return;
                            };
                            detectors::ocr::rgba_2_rgb(frame)
                        };

                        let detected_word = self.ocr_detector.get_strings_from_frame(rgb_frame);

                        let (correct_detection_words, threshold) = {
                            let settings = self.settings.read().unwrap();
                            (
                                settings.sonar.sonar_detection_words.clone(),
                                settings.sonar.sonar_detection_threshold,
                            )
                        };

                        if detectors::ocr::Ocr::is_needed_string(
                            &correct_detection_words,
                            &detected_word,
                            threshold,
                        ) {
                            info!("Detected Word {}", detected_word);
                            self.set_state(BotState::Catch);
                        }
                    }
                    DetectionMethod::Audio => {}
                }
            }
        }
    }

    fn get_difference_mask(&mut self) -> Option<GrayImage> {
        let Some(frame) = self.shared_frame.lock().unwrap().clone() else {
            return None;
        };

        let Some(difference_mask) = self.movemap_detector.get_difference_mask(frame) else {
            return None;
        };

        if self.settings.read().unwrap().movemap.draw_mask {
            let dyn_img = DynamicImage::ImageLuma8(difference_mask.clone());
            *self.force_draw_frame.lock().unwrap() = Some(dyn_img.to_rgba8());
        } else {
            *self.force_draw_frame.lock().unwrap() = None;
        }

        self.egui_ctx.request_repaint();

        Some(difference_mask)
    }

    fn manage_sleep(&mut self, target_fps: u8) {
        let target_duration = Duration::from_secs_f32(1.0 / target_fps as f32);

        let elapsed = self.last_frame_start.elapsed();

        if elapsed < target_duration {
            let sleep_needed = target_duration - elapsed;
            std::thread::sleep(sleep_needed);
        }
    }

    fn set_state(&mut self, new_state: BotState) {
        self.state = new_state;
        let _ = self.tx.send(UiSended::ChangeState(self.state.clone()));
        info!("Bot state: {}", self.state);
    }

    fn handle_commands(&mut self) {
        if let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                BotCommand::Start => {
                    self.start();
                }
                BotCommand::Stop => {
                    self.stop();
                }
            }
        }
    }
}
