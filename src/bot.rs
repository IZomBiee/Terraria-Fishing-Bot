use crate::{controller::Controller, opencv, settings::Settings};
use image::{GrayImage, RgbaImage};
use std::time::{Duration, Instant};
use strum_macros::AsRefStr;

#[derive(Debug, PartialEq, AsRefStr)]
pub enum BotState {
    Idle,
    WaitingForBite,
    Casting,
    Reeling,
    CastingCooldown(Instant),
    CheckingLiquidLevel,
    UsingPotions,
}
pub struct Bot<'a> {
    pub state: BotState,
    pub settings: &'a Settings,
    pub controller: Option<Controller<'a>>,
    liquid_level: Option<u32>,
    last_frame: Option<RgbaImage>,
    last_potion_use_time: Instant,
}

impl<'a> Bot<'a> {
    pub fn new(settings: &'a Settings, controller: Option<Controller<'a>>) -> Bot<'a> {
        Bot {
            state: BotState::Idle,
            settings,
            controller,
            liquid_level: None,
            last_frame: None,
            last_potion_use_time: Instant::now() - Duration::from_hours(1),
        }
    }

    fn set_state(&mut self, new_state: BotState) {
        println!("Bot state: {}", new_state.as_ref());
        self.state = new_state;
    }

    fn get_liquid_level(mask: &GrayImage, threshold: u32) -> Option<u32> {
        for (i, row) in mask.enumerate_rows() {
            let stack: u32 = row.map(|(_x, _y, pixel)| pixel.0[0] as u32).sum();
            if stack > threshold {
                return Some(i);
            }
        }
        None
    }

    fn get_detection_gap(&self) -> Option<(u32, u32)> {
        if let Some(liquid_level) = self.liquid_level {
            let liquid_level = liquid_level as i32 + self.settings.liquid_offset;
            let gap = liquid_level - self.settings.detection_gap_size as i32;
            if liquid_level > 0 && gap > 0 {
                return Some((gap as u32, liquid_level as u32));
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

    pub fn update(&mut self, frame: &RgbaImage) {
        let Some(difference_mask) = &self.get_difference_mask(frame) else {
            self.last_frame = Some(frame.clone());
            return;
        };

        match self.state {
            BotState::UsingPotions => {
                if let Some(controller) = &self.controller
                    && Instant::now() - self.last_potion_use_time > Duration::from_mins(4)
                {
                    controller.use_potions();
                    self.last_potion_use_time = Instant::now();
                }
                self.set_state(BotState::Casting);
            }
            BotState::CheckingLiquidLevel => {
                let level = Bot::get_liquid_level(difference_mask, self.settings.liquid_threshold);
                self.liquid_level = level;

                if self.get_detection_gap().is_some() {
                    println!("The liquid level is {}.", level.unwrap());
                    self.set_state(BotState::UsingPotions);
                } else {
                    println!("Can't find liquid level.");
                };
            }
            BotState::Casting => {
                if let Some(controller) = &self.controller {
                    controller.cast();
                }
                self.set_state(BotState::CastingCooldown(Instant::now()))
            }
            BotState::CastingCooldown(time) => {
                if Instant::now() - time > Duration::from_millis(self.settings.casting_delay_millis)
                {
                    self.set_state(BotState::WaitingForBite);
                };
            }
            BotState::WaitingForBite => {
                let abs_difference = self.get_abs_difference(difference_mask);
                let Some(abs_difference) = abs_difference else {
                    if let Some(controller) = &self.controller {
                        controller.catch();
                    }
                    self.set_state(BotState::CheckingLiquidLevel);
                    return;
                };
                if abs_difference > self.settings.catch_threshold {
                    self.set_state(BotState::Reeling);
                }
            }
            BotState::Reeling => {
                if let Some(controller) = &self.controller {
                    controller.catch();
                }

                self.set_state(BotState::UsingPotions);
            }
            _ => {}
        }
        self.last_frame = Some(frame.clone());
    }

    pub fn draw_detection_gap(&self, frame: &mut RgbaImage) {
        let (width, height) = frame.dimensions();

        if let Some((y_start, y_stop)) = self.get_detection_gap() {
            for (y, color) in [(y_start, [255, 255, 255, 255]), (y_stop, [0, 0, 0, 255])] {
                if y < height {
                    for x in 0..width {
                        frame.put_pixel(x, y, image::Rgba(color));
                    }
                }
            }
        }
    }

    pub fn start(&mut self) {
        if self.state == BotState::Idle {
            println!("Starting bot!");
            self.set_state(BotState::CheckingLiquidLevel);
        }
    }

    pub fn stop(&mut self) {
        if self.state != BotState::Idle {
            println!("Stoping bot!");
            self.set_state(BotState::Idle);
        }
    }
}
