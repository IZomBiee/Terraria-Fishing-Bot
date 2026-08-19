pub mod controller;
pub mod handlers;
pub mod types;

use image::{RgbaImage, buffer::ConvertBuffer};
use types::{BotCommand, BotState, DetectionMethod};

use crate::{
    cursor_capture_server::SharedFrame,
    detectors::{self},
    fps_handler::FpsHandler,
    settings::Settings,
    ui::UiSended,
};

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
    audio_detector: detectors::onnx_audio::OnnxAudio,
    yolo_detector: detectors::onnx_yolo::OnnxYolo,
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
    last_yolo_detection_time: Option<Instant>,
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
        audio_detector: detectors::onnx_audio::OnnxAudio,
        yolo_detector: detectors::onnx_yolo::OnnxYolo,
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
            yolo_detector,
            last_frame_start: Instant::now(),
            fps_handler: FpsHandler::default(),
            last_yolo_detection_time: None,
        }
    }

    pub fn run(&mut self) {
        loop {
            self.last_frame_start = Instant::now();

            self.handle_commands();

            let capture_fps = self.settings.read().unwrap().capture.fps;

            self.update_detectors();
            self.update_state();

            let detection_method = self.settings.read().unwrap().bot.detection_method.clone();

            if detection_method == DetectionMethod::Audio {
                self.manage_sleep(20);
            } else {
                self.manage_sleep(capture_fps);
            }

            self.draw_to_ui();

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
    }

    fn update_detectors(&mut self) {
        let detection_method = self.settings.read().unwrap().bot.detection_method.clone();

        match detection_method {
            DetectionMethod::Audio => {
                self.audio_detector.update();
            }
            DetectionMethod::Ocr => {
                let Some(frame) = self.shared_frame.lock().unwrap().clone() else {
                    return;
                };
                self.ocr_detector.update(frame);
            }
            DetectionMethod::YOLO => {
                let Some(frame) = self.shared_frame.lock().unwrap().clone() else {
                    return;
                };
                self.yolo_detector.update(&frame);
            }
            DetectionMethod::Movemap => {
                let Some(frame) = self.shared_frame.lock().unwrap().clone() else {
                    return;
                };
                self.movemap_detector.update(frame);
            }
            DetectionMethod::YoloAudio => {
                let Some(frame) = self.shared_frame.lock().unwrap().clone() else {
                    return;
                };

                self.audio_detector.update();
                self.yolo_detector.update(&frame);
            }
        }
    }

    fn update_state(&mut self) {
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
                if time.elapsed()
                    > Duration::from_millis(self.settings.read().unwrap().bot.catching_delay_millis)
                {
                    self.set_state(BotState::Cast);
                }
            }
        }
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

    fn draw_to_ui(&self) {
        let detection_method = self.settings.read().unwrap().bot.detection_method.clone();

        match detection_method {
            DetectionMethod::Audio => {
                let Some(waveform_image) = self.audio_detector.render_waveform(600, 600) else {
                    return;
                };

                *self.force_draw_frame.lock().unwrap() = Some(waveform_image);
            }
            DetectionMethod::Ocr => {
                *self.force_draw_frame.lock().unwrap() = self.shared_frame.lock().unwrap().clone();
            }
            DetectionMethod::YOLO => {
                let Some(mut draw_frame) = self.shared_frame.lock().unwrap().clone() else {
                    *self.force_draw_frame.lock().unwrap() = None;
                    return;
                };

                if let Some(detection) = self.yolo_detector.last_detection.clone() {
                    detection.draw(&mut draw_frame);
                };

                *self.force_draw_frame.lock().unwrap() = Some(draw_frame);
            }
            DetectionMethod::Movemap => {
                let mut draw_image: RgbaImage = if self.settings.read().unwrap().movemap.draw_mask {
                    let Some(movemap_image) = self.movemap_detector.last_difference_mask.clone()
                    else {
                        return;
                    };

                    movemap_image.convert()
                } else {
                    let Some(frame) = self.shared_frame.lock().unwrap().clone() else {
                        return;
                    };
                    frame
                };

                if let Some((y0, y1)) = self.movemap_detector.get_liquid_gap() {
                    let (width, height) = draw_image.dimensions();
                    for (y, color) in [(y0, [0, 0, 255, 255]), (y1, [255, 0, 0, 255])] {
                        if y < height {
                            for x in 0..width {
                                draw_image.put_pixel(x, y, image::Rgba(color));
                            }
                        }
                    }
                }

                *self.force_draw_frame.lock().unwrap() = Some(draw_image.convert());
            }
            DetectionMethod::YoloAudio => {
                *self.force_draw_frame.lock().unwrap() = self.shared_frame.lock().unwrap().clone();
            }
        }
    }
}
