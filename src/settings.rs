use crate::bot;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CaptureSettings {
    pub margin: u32,
    pub monitor_id: u8,
    pub fps: u8,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            margin: 200,
            monitor_id: 0,
            fps: 30,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BotSettings {
    pub casting_delay_millis: u64,
    pub use_potions: bool,
    pub cast_max_time: Duration,
    pub detection_method: bot::DetectionMethod,
}

impl Default for BotSettings {
    fn default() -> Self {
        Self {
            casting_delay_millis: 1000,
            use_potions: false,
            cast_max_time: Duration::from_secs(20),
            detection_method: bot::DetectionMethod::MoveMap,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MovemapSettings {
    pub liquid_offset: i32,
    pub liquid_detection_delay_millis: u64,
    pub noises_delay_millis: u64,
    pub detection_threshold: u32,
    pub detection_gap_size: u32,
}

impl Default for MovemapSettings {
    fn default() -> Self {
        Self {
            liquid_offset: -10,
            liquid_detection_delay_millis: 1000,
            noises_delay_millis: 2000,
            detection_threshold: 200,
            detection_gap_size: 30,
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

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Settings {
    pub capture: CaptureSettings,
    pub bot: BotSettings,
    pub movemap: MovemapSettings,
    pub sonar: SonarSettings,
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
