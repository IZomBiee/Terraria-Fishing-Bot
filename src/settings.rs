use serde::{Deserialize, Serialize};
use std::fs;

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
    pub detection_model_path: String,
    pub rec_model_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            margin: 100,
            monitor_id: 1,
            fps: 5,
            casting_delay_millis: 1000,
            liquid_offset: -10,
            liquid_detection_delay_millis: 3000,
            noises_delay_millis: 10000,
            detection_threshold: 400,
            detection_gap_size: 40,
            detection_model_path: "assets\\text-detection-ssfbcj81.onnx".to_owned(),
            rec_model_path: "assets\\text-rec-checkpoint-s52qdbqt.onnx".to_owned(),
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
