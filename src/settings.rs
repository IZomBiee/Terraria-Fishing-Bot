use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub margin: u32,
    pub monitor_id: u8,
    pub fps: u8,
    pub casting_delay_millis: u64,
    pub catch_threshold: u32,
    pub liquid_threshold: u32,
    pub liquid_offset: i32,
    pub detection_gap_size: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            margin: 100,
            monitor_id: 1,
            fps: 5,
            casting_delay_millis: 1000,
            catch_threshold: 200,
            liquid_threshold: 20,
            liquid_offset: -5,
            detection_gap_size: 40,
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
                Self::default()
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
