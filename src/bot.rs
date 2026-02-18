use crate::{BotSettings, controller::Controller, detector::Detector, opencv};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub enum BotState {
    Idle,
    WaitingForBite,
    Casting,
    Reeling,
    CastingCooldown(Duration, Instant),
    ReelingCooldown(Duration, Instant),
}

pub struct Bot {
    pub state: BotState,
    pub detector: Detector,
    pub settings: BotSettings,
    pub controller: Controller,
    last_pos: Option<(u32, u32)>,
}

impl Bot {
    pub fn new(settings: BotSettings, controller: Controller) -> Bot {
        return Bot {
            state: BotState::Idle,
            detector: Detector::new(),
            settings,
            controller,
            last_pos: None,
        };
    }

    pub fn update(&mut self, mask: &[u8], width: u32, height: u32) {
        match self.state {
            BotState::Casting => {
                self.controller.cast();

                self.state = BotState::CastingCooldown(
                    Duration::from_millis(self.settings.casting_delay),
                    Instant::now(),
                )
            }
            BotState::CastingCooldown(duration, time) => {
                if Instant::now() - time > duration {
                    self.state = BotState::WaitingForBite;
                    self.last_pos = None;
                };
            }
            BotState::WaitingForBite => {
                if let Some(current_pos) = self.detector.get_bobber_pos(&mask, width, height) {
                    if let Some(last_pos) = self.last_pos {
                        let dy = (last_pos.1 as i32 - current_pos.1 as i32).abs();
                        if dy > self.settings.catch_thresh as i32 {
                            self.state = BotState::Reeling
                        }
                    }
                    self.last_pos = Some(current_pos);
                }
            }
            BotState::Reeling => {
                self.controller.catch();

                self.state = BotState::ReelingCooldown(
                    Duration::from_millis(self.settings.reeling_delay),
                    Instant::now(),
                );
            }
            BotState::ReelingCooldown(duration, time) => {
                if Instant::now() - time > duration {
                    self.state = BotState::Casting
                };
            }
            _ => {}
        }
    }

    pub fn draw_last_pos(&self, rgba_frame: &mut [u8], width: u32, height: u32) {
        if let Some(last_pos) = self.last_pos {
            opencv::circle(rgba_frame, width, height, last_pos.0, last_pos.1, 15);
        }
    }

    pub fn start(&mut self) {
        println!("Starting bot!");
        self.state = BotState::Casting;
    }

    pub fn stop(&mut self) {
        println!("Stoping bot!");
        self.state = BotState::Idle;
    }
}
