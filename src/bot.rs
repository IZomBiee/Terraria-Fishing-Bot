use crate::{BotSettings, controller::Controller};
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
    CheckingNoise(Instant),
}

pub struct Bot {
    pub state: BotState,
    pub settings: BotSettings,
    pub controller: Option<Controller>,
    liquid_level: Option<u32>,
}

impl Bot {
    pub fn new(settings: BotSettings, controller: Option<Controller>) -> Bot {
        Bot {
            state: BotState::Idle,
            settings,
            controller,
            liquid_level: None,
        }
    }

    fn set_state(&mut self, new_state: BotState) {
        println!("Bot state: {}", new_state.as_ref());
        self.state = new_state;
    }

    fn get_liquid_threshold(&self, width: u32) -> u32 {
        (self.settings.liquid_threshold * width as f32) as u32
    }

    fn get_liquid_offset(&self, height: u32) -> i32 {
        (self.settings.liquid_offset * height as f32) as i32
    }

    fn get_catch_threshold(&self, _height: u32, width: u32) -> u32 {
        let threshold = (self.settings.catch_threshold * width as f32) as u32;
        threshold
    }

    fn get_detection_gap_size(&self, height: u32) -> u32 {
        let gap = (self.settings.detection_gap_size * height as f32) as u32;
        gap
    }

    fn get_liquid_level(mask: &GrayImage, threshold: u32) -> Option<u32>{
        for (i, row) in mask.enumerate_rows() {
            let stack = row.filter(|(_, _, pixel)| pixel.0[0] > 0).count();
            if stack as u32 > threshold {
                return Some(i);
            }
        }
        None
    }

    fn get_detection_gap(&self, height: u32) -> Option<(u32, u32)> {
        if let Some(liquid_level) = self.liquid_level  {
            let liquid_level = liquid_level as i32 + self.get_liquid_offset(height);
            let gap = liquid_level as i32 - self.get_detection_gap_size(height) as i32;
            if liquid_level > 0 && gap > 0 {
                return Some((gap as u32, liquid_level as u32));
            }
        }
        None
    }

    pub fn update(&mut self, mask: &GrayImage) {
        match self.state {
            BotState::CheckingLiquidLevel => {
                let level = Bot::get_liquid_level(mask, self.get_liquid_threshold(mask.width()));
                self.liquid_level = level;

                if self.get_detection_gap(mask.height()).is_some() {
                    self.set_state(BotState::Casting);
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
                if let Some((y_start, y_stop)) = self.get_detection_gap(mask.height()) {
                    let start_idx = (y_start * mask.width()) as usize;
                    let stop_idx = (y_stop * mask.width()) as usize;
                    let abs_difference: u32 =
                        dbg!(mask.as_raw()[start_idx..stop_idx].iter().filter(|pixel| *(*pixel)>0).count() as u32);
                    if abs_difference > self.get_catch_threshold(mask.height(), mask.width()) {
                        self.set_state(BotState::Reeling);
                    }
                } else {
                    if let Some(controller) = &self.controller {
                        controller.catch();
                    }
                    self.set_state(BotState::CheckingLiquidLevel);
                }
            }
            BotState::Reeling => {
                if let Some(controller) = &self.controller {
                    controller.catch();
                }

                self.set_state(BotState::Casting);
            }
            _ => {}
        }
    }

    pub fn draw_detection_gap(&self, frame: &mut RgbaImage) {
        let (width, height) = frame.dimensions();
        
        if let Some((y_start, y_stop)) = self.get_detection_gap(height) {
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