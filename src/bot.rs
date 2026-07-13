use crate::{
    controller::Controller, cursor_capturer::SharedFrame, opencv,
    settings::Settings, sonar_detector::SonarDetector, ui::UiSended,
};

use image::{GrayImage, RgbaImage};
use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

use std::{
    collections::VecDeque,
    fmt::Display,
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, Sender},
    },
    thread::{self},
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, AsRefStr, Clone, Copy)]
pub enum BotState {
    Idle,
    WaitingForBite,
    Cast,
    Catch,
    CastingCooldown(Instant),
    CatchingCooldown(Instant),
    CheckingLiquidLevel(Instant),
    Calibrate(Instant),
}

impl Display for BotState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            BotState::Idle => "Idle",
            BotState::WaitingForBite => "WaitingForBite",
            BotState::Cast => "Cast",
            BotState::Catch => "Catch",
            BotState::CastingCooldown(..) => "CastingCooldown",
            BotState::CatchingCooldown(..) => "CatchingCooldown",
            BotState::CheckingLiquidLevel(..) => "CheckingLiquidLevel",
            BotState::Calibrate(..) => "Calibrate",
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
    MoveMap,
    Audio,
    Sonar,
}

pub struct Bot {
    sonar_detector: SonarDetector,
    shared_frame: SharedFrame,
    force_draw_frame: SharedFrame,
    state: BotState,
    settings: Arc<RwLock<Settings>>,
    rx: Receiver<BotCommand>,
    tx: Sender<UiSended>,
    controller: Controller,
    liquid_levels: Vec<u32>,
    last_frame: Option<RgbaImage>,
    frametime_history: VecDeque<Instant>,
    last_cast_time: Instant,
    calibration_results: Vec<u32>,
    movemap_threshold: Option<u32>,
    shared_frame_count: Arc<RwLock<u64>>,
    last_frame_count: u64,
}

unsafe impl Send for Bot {}

impl Bot {
    pub fn new(
        rx: Receiver<BotCommand>,
        tx: Sender<UiSended>,
        shared_frame: SharedFrame,
        force_draw_frame: SharedFrame,
        settings: Arc<RwLock<Settings>>,
        sonar_detector: SonarDetector,
        shared_frame_count: Arc<RwLock<u64>>,
    ) -> Bot {
        Bot {
            rx,
            tx,
            shared_frame,
            force_draw_frame,
            state: BotState::Idle,
            settings: Arc::clone(&settings),
            controller: Controller::new(Arc::clone(&settings)),
            sonar_detector,
            liquid_levels: Vec::new(),
            last_frame: None,
            frametime_history: VecDeque::new(),
            last_cast_time: Instant::now(),
            movemap_threshold: None,
            calibration_results: Vec::new(),
            shared_frame_count,
            last_frame_count: 0,
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
                let current_frame_count = self.shared_frame_count.read().unwrap().clone();

                let should_update = self.last_frame.as_ref().is_none()
                    || current_frame_count != self.last_frame_count;

                if should_update {
                    self.update(rgba_img);
                    self.update_fps();

                    self.last_frame_count = current_frame_count;
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn set_state(&mut self, new_state: BotState) {
        self.state = new_state;
        let _ = self.tx.send(UiSended::ChangeState(self.state));
        info!("Bot state: {}", self.state);
    }

    fn update_fps(&mut self) {
        let target_fps = self.settings.read().map(|s| s.capture.fps).unwrap_or(30) as i32;
        let count_to_delete = self.frametime_history.len() as i32 - target_fps;
        if count_to_delete > 0 {
            for _ in 0..=count_to_delete {
                self.frametime_history.pop_front();
            }
            let fps = self.get_fps();
            let _ = self.tx.send(UiSended::SetFPS(fps));
        }

        self.frametime_history.push_back(Instant::now());
    }

    pub fn get_fps(&self) -> f32 {
        if self.frametime_history.len() >= 2 {
            let first = self.frametime_history.front().unwrap();

            let last = self.frametime_history.back().unwrap();

            let duration = *last - *first;

            let count = self.frametime_history.len() as f32;

            return count / duration.as_secs_f32();
        }

        0f32
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

    fn get_detection_gap(&self) -> Option<(u32, u32)> {
        let settings = self.settings.read().unwrap();

        if let Some(liquid_level) = self.get_mean_liquid_level() {
            let liquid_level = liquid_level as i32;
            let gap_height = (settings.capture.margin * 2) as f32
                * (settings.movemap.detection_gap_size_precents as f32 / 100.0);
            let gap = liquid_level - (gap_height.round() as i32);
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
                .map(|&pixel| pixel as u32)
                .sum();
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

    fn waiting_for_bite_logic(
        &mut self,
        last_frame: &RgbaImage,
        maybe_difference_mask: Option<&GrayImage>,
    ) {
        let (detection_method, sonar_detection_words, sonar_detection_threshold) = {
            let settings = self.settings.read().unwrap();
            (
                settings.bot.detection_method.clone(),
                settings.sonar.sonar_detection_words.clone(),
                settings.sonar.sonar_detection_threshold,
            )
        };

        match detection_method {
            DetectionMethod::MoveMap => {
                let Some(difference_mask) = maybe_difference_mask else {
                    return;
                };

                let abs_difference = self.get_abs_difference(difference_mask);

                let Some(abs_difference) = abs_difference else {
                    info!("Could not read movement! Recalibrate.");
                    self.controller.catch();
                    self.set_state(BotState::CheckingLiquidLevel(Instant::now()));
                    return;
                };

                if abs_difference > self.movemap_threshold.unwrap() {
                    info!(
                        "Movement detected ({}/{}).",
                        abs_difference,
                        self.movemap_threshold.unwrap()
                    );
                    self.set_state(BotState::Catch);
                }
            }
            DetectionMethod::Sonar => {
                let word = self
                    .sonar_detector
                    .get_strings_from_frame(opencv::rgba_2_rgb(last_frame));

                if sonar_detection_words.split(",").any(|string| {
                    SonarDetector::is_needed_string(
                        string,
                        &word,
                        sonar_detection_threshold,
                    )
                }) {
                    let _ = self.tx.send(UiSended::DetectedItem(word));
                    self.set_state(BotState::Catch);
                }
            }

            DetectionMethod::Audio => {
                
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
        let mut maybe_difference_mask: Option<GrayImage> = None;

        let settings = self.settings.read().unwrap().clone();

        if settings.bot.detection_method == DetectionMethod::MoveMap {
            maybe_difference_mask = self.get_difference_mask(&frame);
        }

        if settings.movemap.draw_mask
            && let Some(difference_mask) = maybe_difference_mask.as_ref()
        {
            let rgba_frame: image::RgbaImage =
                image::DynamicImage::ImageLuma8(difference_mask.clone()).to_rgba8();

            if let Ok(mut lock) = self.force_draw_frame.lock() {
                *lock = Some(rgba_frame);
            }
        } else if settings.bot.detection_method == DetectionMethod::Audio {
            if let Ok(mut lock) = self.force_draw_frame.lock() {
                *lock = None;
            }
        }

        match self.state {
            BotState::Idle => {}
            BotState::CheckingLiquidLevel(time) => {
                if let Some(difference_mask) = maybe_difference_mask.as_ref() {
                    if let Some(level) = Bot::get_liquid_level(&difference_mask) {
                        self.liquid_levels.push(level);
                        self.liquid_levels.sort();
                    }
                }

                let delay = self
                    .settings
                    .read()
                    .unwrap()
                    .movemap
                    .liquid_detection_delay_ms;

                if time.elapsed() > Duration::from_millis(delay)
                    && let Some(mean_liquid_level) = self.get_mean_liquid_level()
                {
                    info!("The liquid level is: {}", mean_liquid_level);
                    if let Some(gap) = self.get_detection_gap() {
                        let _ = self.tx.send(UiSended::LiquidGap(gap.0, gap.1));
                    }
                    self.set_state(BotState::Cast);
                }
            }

            BotState::Cast => {
                self.controller.cast();
                self.last_cast_time = Instant::now();

                if self.settings.read().unwrap().bot.use_potions {
                    self.controller.use_potions_if_necessary();
                }

                self.set_state(BotState::CastingCooldown(Instant::now()));
            }

            BotState::CastingCooldown(time) => {
                let delay = self.settings.read().unwrap().bot.casting_delay_millis;
                let detection_method = self.settings.read().unwrap().bot.detection_method.clone();
                if time.elapsed() > Duration::from_millis(delay) {
                    if self.movemap_threshold.is_some()
                        || detection_method != DetectionMethod::MoveMap
                    {
                        self.set_state(BotState::WaitingForBite);
                    } else {
                        self.calibration_results.clear();
                        self.set_state(BotState::Calibrate(Instant::now()));
                    }
                }
            }

            BotState::Calibrate(time) => {
                if time.elapsed()
                    < Duration::from_millis(
                        self.settings.read().unwrap().movemap.calibration_time_ms,
                    )
                {
                    let Some(difference_mask) = maybe_difference_mask.as_ref() else {
                        return;
                    };

                    let abs_difference = self.get_abs_difference(difference_mask);

                    let Some(abs_difference) = abs_difference else {
                        return;
                    };

                    self.calibration_results.push(abs_difference);
                } else {
                    let calibration_multiplier = (self
                        .settings
                        .read()
                        .unwrap()
                        .movemap
                        .calibration_multiplier_precents
                        as f32)
                        / 100.0;
                    let calibration_result =
                        ((self.calibration_results.iter().max().cloned().unwrap() as f32)
                            * calibration_multiplier) as u32;

                    info!("Calibrated threshold is {}.", calibration_result);

                    self.movemap_threshold = Some(calibration_result);

                    self.set_state(BotState::WaitingForBite);
                }
            }

            BotState::WaitingForBite => {
                self.waiting_for_bite_logic(&frame, maybe_difference_mask.as_ref());

                if self.last_cast_time.elapsed() > Duration::from_secs(30) {
                    info!(
                        "Waiting for bite for too long ({} s)! Recast.",
                        self.last_cast_time.elapsed().as_secs()
                    );
                    self.set_state(BotState::Catch);
                }
            }

            BotState::Catch => {
                self.controller.catch();
                self.set_state(BotState::CatchingCooldown(Instant::now()));
            }

            BotState::CatchingCooldown(time) => {
                let delay = self.settings.read().unwrap().bot.catching_delay_millis;
                if time.elapsed() > Duration::from_millis(delay) {
                    self.set_state(BotState::Cast);
                }
            }
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
                settings.bot.detection_method.clone()
            };

            match detection_method {
                DetectionMethod::MoveMap => {
                    self.set_state(BotState::CheckingLiquidLevel(Instant::now()));
                }
                DetectionMethod::Sonar => {
                    self.set_state(BotState::Cast);
                }
                DetectionMethod::Audio => {
                    self.set_state(BotState::Cast);
                }
            }

            self.movemap_threshold = None;

            return true;
        }
        false
    }

    pub fn stop(&mut self) {
        if self.state != BotState::Idle {
            info!("Stoping bot");
            self.liquid_levels.clear();
            self.set_state(BotState::Idle);
        }
    }
}
