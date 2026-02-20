use crate::{BotSettings, controller::Controller, detector::Detector, opencv};
use std::time::{Duration, Instant};
use strum_macros::AsRefStr;

#[derive(Debug, PartialEq, AsRefStr)]
pub enum BotState {
    Idle,
    WaitingForBite,
    Casting,
    Reeling,
    CastingCooldown(Duration, Instant),
    CheckingLiquidLevel,
}

pub struct Bot {
    pub state: BotState,
    pub settings: BotSettings,
    pub controller: Controller,
    liquid_level: i32,
}

impl Bot {
    pub fn new(settings: BotSettings, controller: Controller) -> Bot {
        return Bot {
            state: BotState::Idle,
            settings,
            controller,
            liquid_level: -1,
        };
    }

    fn set_state(&mut self, new_state: BotState) {
        println!("Bot state: {}", new_state.as_ref());
        self.state = new_state;
    }

    fn get_liquid_level(&self, mask: &[u8]) -> i32 {
        let mut stack = 0;
        for (index, pixel) in mask.iter().enumerate() {
            if *pixel > 0 {
                stack += 1;
            }

            if stack > self.settings.liquid_threshold {
                return index as i32;
            }
        }
        return -1;
    }

    pub fn update(&mut self, mask: &[u8]) {
        match self.state {
            BotState::CheckingLiquidLevel => {
                self.liquid_level = self.get_liquid_level(mask);
                self.set_state(BotState::Casting);
            }
            BotState::Casting => {
                self.controller.cast();

                self.set_state(BotState::CastingCooldown(
                    Duration::from_millis(self.settings.casting_delay),
                    Instant::now(),
                ))
            }
            BotState::CastingCooldown(duration, time) => {
                if Instant::now() - time > duration {
                    self.set_state(BotState::WaitingForBite);
                };
            }
            BotState::WaitingForBite => {
                if self.liquid_level == -1 {
                    self.set_state(BotState::Reeling);
                } else {
                    let abs_difference: u32 = mask[..self.liquid_level as usize]
                        .iter()
                        .map(|&x| x as u32)
                        .sum();
                    if abs_difference > self.settings.catch_threshold * 255 {
                        self.set_state(BotState::Reeling);
                    }
                }
            }
            BotState::Reeling => {
                self.controller.catch();

                self.set_state(BotState::Casting);
            }
            _ => {}
        }
    }

    pub fn draw_liquid_level(&self, rgba_frame: &mut [u8], width: u32, height: u32) {
        if self.liquid_level == -1 {
            return;
        }

        let y = (self.liquid_level.clone() as u32) / width;

        if y >= height {
            return;
        }

        let bytes_per_row = (width * 4) as usize;
        let start = y as usize * bytes_per_row;
        let stop = start + bytes_per_row;

        if let Some(row_slice) = rgba_frame.get_mut(start..stop) {
            for pixel in row_slice.chunks_exact_mut(4) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 255;
                pixel[3] = 255;
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
