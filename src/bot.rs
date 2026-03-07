use crate::{
    controller::Controller, cursor_capturer::SharedFrame, opencv, settings::Settings,
    sonar_detector::SonarDetector, ui::UiSended,
};

use image::{GrayImage, RgbaImage};
use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

use std::{
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, Sender},
    },
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, AsRefStr, Clone, Copy)]
pub enum BotState {
    Idle,
    WaitingForBite,
    CheckingBite,
    Cast,
    Catch,
    CastingCooldown(Instant),
    CheckingLiquidLevel(Instant),
    CheckingNoise(Instant),
}

pub enum BotCommand {
    Start,
    Stop,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum DetectionMethod {
    MoveMap,
    Yolo,
    Sonar,
}

pub struct Bot {
    sonar_detector: SonarDetector,
    shared_frame: SharedFrame,
    state: BotState,
    settings: Arc<RwLock<Settings>>,
    rx: Receiver<BotCommand>,
    tx: Sender<UiSended>,
    controller: Controller,
    liquid_levels: Vec<u32>,
    noises: Vec<u32>,
    last_frame: Option<RgbaImage>,
    last_cast_time: Option<Instant>,
}

impl Bot {
    pub fn new(
        rx: Receiver<BotCommand>,
        tx: Sender<UiSended>,
        shared_frame: SharedFrame,
        settings: Arc<RwLock<Settings>>,
        sonar_detector: SonarDetector,
    ) -> Bot {
        Bot {
            rx,
            tx,
            shared_frame,
            state: BotState::Idle,
            settings: Arc::clone(&settings),
            controller: Controller::new(settings),
            sonar_detector,
            liquid_levels: Vec::new(),
            noises: Vec::new(),
            last_frame: None,
            last_cast_time: None,
        }
    }

    pub fn run(&mut self) {
        loop {
            self.handle_commands();

            let maybe_img = self
                .shared_frame
                .lock()
                .ok()
                .and_then(|guard| guard.clone());

            if let Some(rgba_img) = maybe_img {
                let should_update = self
                    .last_frame
                    .as_ref()
                    .is_none_or(|last| *last != rgba_img);

                if should_update {
                    self.update(rgba_img);
                }
            }

            sleep(Duration::from_millis(1000 / 60));
        }
    }

    fn set_state(&mut self, new_state: BotState) {
        self.state = new_state;
        let _ = self.tx.send(UiSended::ChangeState(self.state));
        info!("Bot state: {:?}", self.state);
    }

    fn get_liquid_level(mask: &GrayImage) -> Option<u32> {
        let mut max_stack = 0;
        let mut max_stack_y: Option<u32> = None;
        for (i, row) in mask.enumerate_rows() {
            let stack: u32 = row.map(|(_x, _y, pixel)| pixel.0[0] as u32).sum();
            if stack > max_stack {
                max_stack = stack;
                max_stack_y = Some(i);
            }
        }
        max_stack_y
    }

    fn get_mean_liquid_level(&self) -> Option<u32> {
        let count = self.liquid_levels.len() as u32;
        if count <= 3 {
            return None;
        };

        let mean: u32 = if count.is_multiple_of(2) {
            (self.liquid_levels[(count as f32 / 2f32) as usize]
                + self.liquid_levels[((count as f32 / 2f32) as usize) + 1])
                / 2
        } else {
            self.liquid_levels[(count as f32 / 2f32) as usize]
        };
        Some(mean)
    }

    fn get_max_noise_level(&self) -> Option<u32> {
        self.noises.iter().max().cloned()
    }

    fn get_detection_gap(&self) -> Option<(u32, u32)> {
        let settings = self.settings.read().unwrap();

        if let Some(liquid_level) = self.get_mean_liquid_level() {
            let liquid_level = liquid_level as i32 + settings.liquid_offset;
            let gap = liquid_level - settings.detection_gap_size as i32;
            if liquid_level > 0 {
                return Some((std::cmp::max(gap, 0) as u32, liquid_level as u32));
            }
        }
        None
    }

    fn get_abs_difference(&self, mask: &GrayImage) -> Option<u32> {
        if let Some((y_start, y_stop)) = self.get_detection_gap() {
            let start_idx = (y_start * mask.width()) as usize;
            let stop_idx = (y_stop * mask.width()) as usize;
            let abs_difference: u32 = mask.as_raw()[start_idx..stop_idx]
                .iter()
                .filter(|pixel| *(*pixel) > 0)
                .count() as u32;
            return Some(abs_difference);
        }
        None
    }

    pub fn get_difference_mask(&self, current_frame: &RgbaImage) -> Option<GrayImage> {
        let last_image = self.last_frame.as_ref()?;
        if current_frame.dimensions() != last_image.dimensions() {
            return None;
        }
        Some(opencv::rgba_difference_mask(current_frame, last_image))
    }

    fn waiting_for_bite_logic(&mut self, last_frame: &RgbaImage, difference_mask: &GrayImage) {
        let (
            detection_method,
            detection_threshold,
            sonar_detector_words,
            sonar_detector_threshold,
            cast_max_time,
            use_potions,
        ) = {
            let settings = self.settings.read().unwrap();

            (
                settings.detection_method.clone(),
                settings.detection_threshold,
                settings.sonar_detection_words.clone(),
                settings.sonar_detection_threshold,
                settings.cast_max_time,
                settings.use_potions,
            )
        };

        match detection_method {
            DetectionMethod::MoveMap => {
                let abs_difference = self.get_abs_difference(difference_mask);

                let Some(abs_difference) = abs_difference else {
                    self.controller.catch();
                    self.set_state(BotState::CheckingLiquidLevel(Instant::now()));
                    return;
                };

                if let Some(noise) = self.get_max_noise_level() {
                    if abs_difference > noise + detection_threshold {
                        self.set_state(BotState::Catch);
                    }
                } else {
                    self.controller.catch();
                    self.set_state(BotState::CheckingNoise(Instant::now()));
                }
            }
            DetectionMethod::Sonar => {
                let words = self
                    .sonar_detector
                    .get_strings_from_frame(opencv::rgba_2_rgb(last_frame));

                if sonar_detector_words.split(",").any(|string| {
                    SonarDetector::is_needed_string(string, words.clone(), sonar_detector_threshold)
                }) {
                    let _ = self.tx.send(UiSended::DetectedItems(words));
                    self.set_state(BotState::Catch);
                }

                if let Some(time) = self.last_cast_time
                    && time.elapsed() > cast_max_time
                    && use_potions
                {
                    self.controller.use_potions();
                    self.last_cast_time = Some(Instant::now());
                }
            }
            _ => {
                todo!()
            }
        }
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

    pub fn update(&mut self, frame: RgbaImage) {
        let Some(difference_mask) = self.get_difference_mask(&frame) else {
            self.last_frame = Some(frame.clone());
            return;
        };

        match self.state {
            BotState::CheckingLiquidLevel(time) => {
                if let Some(level) = Bot::get_liquid_level(&difference_mask) {
                    self.liquid_levels.push(level);
                    self.liquid_levels.sort();
                }

                let delay = self.settings.read().unwrap().liquid_detection_delay_millis;

                if time.elapsed() > Duration::from_millis(delay)
                    && let Some(mean_liquid_level) = self.get_mean_liquid_level()
                {
                    info!("The liquid level is: {}", mean_liquid_level);
                    if let Some(gap) = self.get_detection_gap() {
                        let _ = self.tx.send(UiSended::LiquidGap(gap.0, gap.1));
                    }
                    self.set_state(BotState::CheckingNoise(Instant::now()));
                }
            }

            BotState::CheckingNoise(time) => {
                if let Some(level) = self.get_abs_difference(&difference_mask) {
                    self.noises.push(level);
                }

                let delay = self.settings.read().unwrap().noises_delay_millis;

                if time.elapsed() > Duration::from_millis(delay)
                    && let Some(max_noise_level) = self.get_max_noise_level()
                {
                    info!("The noise level is: {}", max_noise_level);

                    self.set_state(BotState::Cast);
                }
            }

            BotState::Cast => {
                self.controller.cast();
                if self.settings.read().unwrap().use_potions {
                    self.controller.use_potions();
                }

                self.set_state(BotState::CastingCooldown(Instant::now()));
            }

            BotState::CastingCooldown(time) => {
                let delay = self.settings.read().unwrap().casting_delay_millis;
                if time.elapsed() > Duration::from_millis(delay) {
                    self.set_state(BotState::WaitingForBite);
                }
            }

            BotState::WaitingForBite => {
                self.waiting_for_bite_logic(&frame, &difference_mask);
            }

            BotState::Catch => {
                self.controller.catch();
                self.set_state(BotState::Cast);
            }

            _ => {}
        }

        self.last_frame = Some(frame);
    }

    pub fn draw_detection_gap(&self, frame: &mut RgbaImage) {
        let (width, height) = frame.dimensions();

        if let Some((y_start, y_stop)) = self.get_detection_gap() {
            for (y, color) in [(y_start, [0, 0, 255, 255]), (y_stop, [255, 0, 0, 255])] {
                if y < height {
                    for x in 0..width {
                        frame.put_pixel(x, y, image::Rgba(color));
                    }
                }
            }
        }
    }

    pub fn start(&mut self) -> bool {
        if self.state == BotState::Idle {
            info!("Starting bot");

            let detection_method = {
                let settings = self.settings.read().unwrap();
                settings.detection_method.clone()
            };

            match detection_method {
                DetectionMethod::MoveMap => {
                    self.set_state(BotState::CheckingLiquidLevel(Instant::now()));
                }
                DetectionMethod::Sonar => {
                    self.set_state(BotState::Cast);
                }
                _ => {
                    todo!();
                }
            }

            return true;
        }
        false
    }

    pub fn stop(&mut self) {
        if self.state != BotState::Idle {
            info!("Stoping bot");
            self.liquid_levels.clear();
            self.noises.clear();
            self.last_cast_time = None;
            self.set_state(BotState::Idle);
        }
    }
}
