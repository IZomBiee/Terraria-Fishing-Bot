use crate::{BotSettings, controller::Controller};
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
    pub controller: Controller,
    liquid_level: u32,
}

impl Bot {
    pub fn new(settings: BotSettings, controller: Controller) -> Bot {
        return Bot {
            state: BotState::Idle,
            settings,
            controller,
            liquid_level: 0,
        };
    }

    fn set_state(&mut self, new_state: BotState) {
        println!("Bot state: {}", new_state.as_ref());
        self.state = new_state;
    }

    fn update_liquid_level(&mut self, mask: &[u8], width: u32, height: u32) {
        for y in 0..height {
            let mut line_stack = 0;
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let pixel = mask[idx];
                if pixel > 0 {
                    line_stack += 1;
                }
            }
            if line_stack > self.settings.liquid_threshold {
                let mut level = y as i32 + self.settings.liquid_offset;
                if level < 0 {
                    level = 0
                }
                self.liquid_level = level as u32;
                return;
            }
        }
        self.liquid_level = 0;
    }

    fn get_detection_gap(&self) -> Option<(u32, u32)> {
        let gap = self.liquid_level as i32 - self.settings.liquid_gap as i32;
        if self.liquid_level > 0 && gap > 0 {
            return Some((gap as u32, self.liquid_level));
        }
        None
    }

    pub fn update(&mut self, mask: &[u8], width: u32, height: u32) {
        match self.state {
            BotState::CheckingLiquidLevel => {
                self.update_liquid_level(mask, width, height);
                if self.get_detection_gap().is_some() {
                    self.set_state(BotState::Casting);
                };
            }
            BotState::Casting => {
                self.controller.cast();

                self.set_state(BotState::CastingCooldown(Instant::now()))
            }
            BotState::CastingCooldown(time) => {
                if Instant::now() - time > Duration::from_millis(self.settings.casting_delay) {
                    self.set_state(BotState::WaitingForBite);
                };
            }
            BotState::WaitingForBite => {
                if let Some((y_start, y_stop)) = self.get_detection_gap() {
                    let start_idx = (y_start * width) as usize;
                    let stop_idx = (y_stop * width) as usize;
                    let abs_difference: u32 =
                        mask[start_idx..stop_idx].iter().map(|&x| x as u32).sum();

                    if abs_difference > self.settings.catch_threshold * 255 {
                        self.set_state(BotState::Reeling);
                    }
                } else {
                    self.controller.catch();
                    self.set_state(BotState::CheckingLiquidLevel);
                }
            }
            BotState::Reeling => {
                self.controller.catch();

                self.set_state(BotState::Casting);
            }
            _ => {}
        }
    }

    pub fn draw_detection_gap(&self, rgba_frame: &mut [u8], width: u32, _height: u32) {
        if let Some((y_start, y_stop)) = self.get_detection_gap() {
            for (y, color) in [(y_start, [255, 255, 255]), (y_stop, [0, 0, 0])] {
                let bytes_per_row = (width * 4) as usize;
                let start = y as usize * bytes_per_row;
                let stop = start + bytes_per_row;

                if let Some(row_slice) = rgba_frame.get_mut(start..stop) {
                    for pixel in row_slice.chunks_exact_mut(4) {
                        pixel[0] = color[0];
                        pixel[1] = color[1];
                        pixel[2] = color[2];
                        pixel[3] = 255;
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
