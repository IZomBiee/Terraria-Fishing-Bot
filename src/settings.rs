use crate::bot;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CaptureSettings {
    pub margin: u32,
    pub fps: u8,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            margin: 150,
            fps: 30,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BotSettings {
    pub casting_delay_millis: u64,
    pub catching_delay_millis: u64,
    pub use_potions: bool,
    pub potion_use_delay: Duration,
    pub detection_method: bot::DetectionMethod,
    pub bot_toggle_hotkey: Option<String>,
    bot_target_fps: u8
}

impl Default for BotSettings {
    fn default() -> Self {
        Self {
            casting_delay_millis: 1000,
            catching_delay_millis: 300,
            use_potions: false,
            potion_use_delay: Duration::from_secs(20),
            detection_method: bot::DetectionMethod::MoveMap,
            bot_toggle_hotkey: Some("P".to_owned()),
            bot_target_fps: 30,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MovemapSettings {
    pub liquid_detection_delay_ms: u64,
    pub detection_gap_size_precents: u8,
    pub calibration_time_ms: u64,
    pub calibration_multiplier_precents: u8,
    pub draw_mask: bool,
}

impl Default for MovemapSettings {
    fn default() -> Self {
        Self {
            liquid_detection_delay_ms: 1000,
            detection_gap_size_precents: 30,
            calibration_time_ms: 7000,
            calibration_multiplier_precents: 60,
            draw_mask: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SonarSettings {
    pub sonar_detection_threshold: usize,
    pub sonar_detection_words: String,
}

impl Default for SonarSettings {
    fn default() -> Self {
        Self {
            sonar_detection_threshold: 4,
            sonar_detection_words: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AudioSettings {
    pub min_confidence: u8,
    pub sample_rate: u32,
    pub buffer_duration: Duration,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            min_confidence: 50,
            sample_rate: 22050,
            buffer_duration: Duration::from_secs(1),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Settings {
    pub capture: CaptureSettings,
    pub bot: BotSettings,
    pub movemap: MovemapSettings,
    pub sonar: SonarSettings,
    pub audio: AudioSettings,
}

impl Settings {
    pub fn load_from_file(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(file) => {
                    info!("Settings loaded");
                    file
                }
                Err(e) => {
                    error!("Can't read settings because of: {e}. Using defaults");
                    Self::default()
                }
            },
            Err(e) => {
                error!("Can't load settings because of: {e}. Using defaults");
                Self::default()
            }
        }
    }

    pub fn save_to_file(&self, path: &str) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(path, json) {
                    error!("Can't write settings because of: {e}");
                } else {
                    info!("Settings sucessfully saved");
                }
            }
            Err(e) => {
                error!("Can't save settings because of: {e}");
            }
        }
    }
}
