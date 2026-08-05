pub mod controller;
pub mod handlers;
pub mod types;

use types::{BotCommand, BotState, DetectionMethod};

use crate::{
    cursor_capture_server::SharedFrame,
    detectors::{self},
    fps_handler::FpsHandler,
    settings::Settings,
    ui::UiSended,
};

use image::{DynamicImage, GrayImage};
use log::info;

use std::{
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, Sender},
    },
    time::{Duration, Instant},
};

pub struct Bot {
    ocr_detector: detectors::ocr::Ocr,
    audio_detector: detectors::onnx_audio::ONNXAudio,
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
        audio_detector: detectors::onnx_audio::ONNXAudio,
        movemap_detector: detectors::movemap::Movemap,
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
                let waveform_image = self.audio_detector.render_waveform(600, 600);
                if waveform_image.is_some() {
                    *self.force_draw_frame.lock().unwrap() = waveform_image;
                }
                self.manage_sleep(30);
            } else {
                self.manage_sleep(capture_fps);
                if !self.settings.read().unwrap().movemap.draw_mask {
                    *self.force_draw_frame.lock().unwrap() = None;
                }
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
            BotState::LiquidLevelCalibration(time) => self.handle_liquid_calibration(time),
            BotState::MovemapThresholdCalibration(time) => self.handle_threshold_calibration(time),
            BotState::WaitingForBite(time) => self.handle_waiting_for_bite(time),
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
                    self.get_and_show_difference_mask();
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
                    self.get_and_show_difference_mask();
                }
                if time.elapsed()
                    > Duration::from_millis(self.settings.read().unwrap().bot.catching_delay_millis)
                {
                    self.set_state(BotState::Cast);
                }
            }
        }
    }

    fn get_and_show_difference_mask(&mut self) -> Option<GrayImage> {
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
