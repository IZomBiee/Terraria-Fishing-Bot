use crate::{controller::Controller, opencv, settings::Settings, sonar_detector::SonarDetector};
use image::{GrayImage, RgbaImage};
use std::time::{Duration, Instant};
use strum_macros::AsRefStr;

#[derive(Debug, PartialEq, AsRefStr)]
pub enum BotState {
    Idle,
    WaitingForBite,
    CheckingBite,
    Casting,
    Reeling,
    CastingCooldown(Instant),
    CheckingLiquidLevel(Instant),
    CheckingNoise(Instant),
    UsingPotions,
}
pub struct Bot<'a> {
    pub state: BotState,
    pub settings: &'a Settings,
    pub controller: Controller<'a>,
    pub sonar_detector: Option<SonarDetector<'a>>,
    liquid_levels: Vec<u32>,
    noises: Vec<u32>,
    last_frame: Option<RgbaImage>,
    last_potion_use_time: Option<Instant>,
}

impl<'a> Bot<'a> {
    pub fn new(
        settings: &'a Settings,
        controller: Controller<'a>,
        sonar_detector: Option<SonarDetector<'a>>,
    ) -> Bot<'a> {
        Bot {
            state: BotState::Idle,
            settings,
            controller,
            sonar_detector,
            liquid_levels: Vec::new(),
            noises: Vec::new(),
            last_frame: None,
            last_potion_use_time: None,
        }
    }

    fn set_state(&mut self, new_state: BotState) {
        println!("Bot state: {}", new_state.as_ref());
        self.state = new_state;
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
        if let Some(liquid_level) = self.get_mean_liquid_level() {
            let liquid_level = liquid_level as i32 + self.settings.liquid_offset;
            let gap = liquid_level - self.settings.detection_gap_size as i32;
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

    pub fn update(&mut self, frame: &RgbaImage) {
        let Some(difference_mask) = &self.get_difference_mask(frame) else {
            self.last_frame = Some(frame.clone());
            return;
        };

        match self.state {
            BotState::CheckingLiquidLevel(time) => {
                if let Some(level) = Bot::get_liquid_level(difference_mask) {
                    self.liquid_levels.push(level);
                    self.liquid_levels.sort();
                }

                if Instant::now() - time
                    > Duration::from_millis(self.settings.liquid_detection_delay_millis)
                {
                    if let Some(mean_liquid_level) = self.get_mean_liquid_level() {
                        println!("The liquid level is {}.", mean_liquid_level);
                        self.set_state(BotState::CheckingNoise(Instant::now()));
                    } else {
                        println!("Can't find liquid level.");
                    }
                }
            }
            BotState::CheckingNoise(time) => {
                if let Some(level) = self.get_abs_difference(difference_mask) {
                    self.noises.push(level);
                }

                if Instant::now() - time > Duration::from_millis(self.settings.noises_delay_millis)
                {
                    if let Some(max_noise_level) = self.get_max_noise_level() {
                        println!("The noise level is {}.", max_noise_level);
                        self.set_state(BotState::UsingPotions);
                    } else {
                        println!("Can't find noise level.");
                    }
                }
            }
            BotState::UsingPotions => {
                if self.settings.use_potions {
                    if let Some(last_potion_use_time) = self.last_potion_use_time {
                        if Instant::now() - last_potion_use_time > Duration::from_mins(4) {
                            self.controller.use_potions();
                            self.last_potion_use_time = Some(Instant::now());
                        }
                    } else {
                        self.controller.use_potions();
                        self.last_potion_use_time = Some(Instant::now());
                    }
                }

                self.set_state(BotState::Casting);
            }
            BotState::Casting => {
                self.controller.cast();
                self.set_state(BotState::CastingCooldown(Instant::now()))
            }
            BotState::CastingCooldown(time) => {
                if Instant::now() - time > Duration::from_millis(self.settings.casting_delay_millis)
                {
                    self.set_state(BotState::WaitingForBite);
                };
            }
            BotState::WaitingForBite => {
                if let Some(detector) = &mut self.sonar_detector {
                    let rgb_frame = &opencv::rgba_2_rgb(frame);
                    let text = detector.get_text_from_frame(rgb_frame);
                    println!("Text: {:?}", text);
                } else {
                    let abs_difference = self.get_abs_difference(difference_mask);

                    let Some(abs_difference) = abs_difference else {
                        println!("No liquid level!");
                        self.controller.catch();
                        self.set_state(BotState::CheckingLiquidLevel(Instant::now()));
                        return;
                    };

                    if let Some(noise) = self.get_max_noise_level() {
                        if abs_difference > noise + self.settings.detection_threshold {
                            println!(
                                "Difference: {} Noise: {} Threshold: {}",
                                abs_difference,
                                noise,
                                noise + self.settings.detection_threshold
                            );
                            self.set_state(BotState::Reeling);
                        }
                    } else {
                        println!("No noise level!");
                        self.controller.catch();
                        self.set_state(BotState::CheckingNoise(Instant::now()));
                        return;
                    }
                }
            }
            BotState::Reeling => {
                self.controller.catch();

                self.set_state(BotState::UsingPotions);
            }
            _ => {}
        }
        self.last_frame = Some(frame.clone());
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
            println!("Starting bot!");
            if self.sonar_detector.is_none() {
                self.set_state(BotState::CheckingLiquidLevel(Instant::now()));
            } else {
                self.set_state(BotState::UsingPotions);
            }
            return true;
        }
        false
    }

    pub fn stop(&mut self) {
        if self.state != BotState::Idle {
            println!("Stoping bot!");
            self.liquid_levels.clear();
            self.noises.clear();
            self.set_state(BotState::Idle);
        }
    }
}
