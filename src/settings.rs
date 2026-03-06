use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};

use crate::bot;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub margin: u32,
    pub monitor_id: u8,
    pub fps: u8,
    pub casting_delay_millis: u64,
    pub liquid_offset: i32,
    pub liquid_detection_delay_millis: u64,
    pub noises_delay_millis: u64,
    pub detection_threshold: u32,
    pub detection_gap_size: u32,
    pub use_sonar: bool,
    pub use_potions: bool,
    pub detection_method: bot::DetectionMethod,
    pub sonar_detection_threshold: usize,
    pub sonar_detection_words: String,
    pub cast_max_time: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            margin: 100,
            monitor_id: 1,
            fps: 30,
            casting_delay_millis: 1000,
            liquid_offset: -5,
            liquid_detection_delay_millis: 500,
            noises_delay_millis: 2000,
            detection_threshold: 150,
            detection_gap_size: 20,
            use_sonar: false,
            use_potions: false,
            detection_method: bot::DetectionMethod::MoveMap,
            sonar_detection_threshold: 5,
            sonar_detection_words: String::new(),
            cast_max_time: Duration::from_secs(20),
        }
    }
}

impl Settings {
    pub fn load_from_file(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|err| {
                println!("Failed to parse {}: {}. Using defaults.", path, err);
                Self::default()
            }),
            Err(_) => {
                println!("Can't read {}, using defaults.", path);
                let default = Self::default();
                default.save_to_file(path);
                default
            }
        }
    }

    pub fn save_to_file(&self, path: &str) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(path, json) {
                    println!("Failed to write to {}: {}", path, e);
                } else {
                    println!("Settings successfully saved to {}", path);
                }
            }
            Err(e) => println!("Failed to serialize settings: {}", e),
        }
    }
}
