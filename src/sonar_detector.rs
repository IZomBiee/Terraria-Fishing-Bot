#![allow(warnings)]

use image::{GrayImage, RgbImage};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use settings::Settings;
use std::{fs, io::Read, str::FromStr};

use crate::settings;

pub struct SonarDetector {
    settings: Settings,
    engine: OcrEngine,
}

impl SonarDetector {
    pub fn new(settings: Settings) -> Self {
        let detection_model_path = "assets\\text-detection-ssfbcj81.onnx";
        let detection_model_data =
            fs::read(detection_model_path).expect(&format!("Can't load {}!", detection_model_path));

        let rec_model_path = "assets\\text-rec-checkpoint-s52qdbqt.onnx";
        let rec_model_data =
            fs::read(rec_model_path).expect(&format!("Can't load {}!", rec_model_path));

        let detection_model = Model::load(detection_model_data).unwrap();
        let rec_model = Model::load(rec_model_data).unwrap();

        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection_model),
            recognition_model: Some(rec_model),
            ..Default::default()
        })
        .unwrap();

        return SonarDetector { settings, engine };
    }

    pub fn get_text_from_frame(&mut self, frame: &RgbImage) -> Vec<String> {
        let img_source = ImageSource::from_bytes(frame.as_raw(), frame.dimensions()).unwrap();
        let input = self.engine.prepare_input(img_source).unwrap();
        let text = self.engine.get_text(&input).unwrap_or(String::new());
        text.split(" ")
            .map(|elem| elem.to_owned().to_lowercase())
            .collect()
    }

    pub fn is_needed_item(&mut self, frame: &RgbImage) -> bool {
        let text = self.get_text_from_frame(frame);

        return false;
    }
}

// mod tests {
//     use super::*;

//     #[test]
//     fn find_bass() {
//         let settings = Settings::default();
//         let mut detector = SonarDetector::new(&settings);

//         let img = image::open("assets\\sonar_bass.png")
//             .expect("Can't load image!")
//             .to_rgb8();

//         let words = detector.get_text_from_frame(&img);

//         assert!(
//             words.contains(&"bass".to_owned()),
//             "Not founded \"bass\" on image but found {:?}.",
//             words
//         );
//     }

//     #[test]
//     fn find_bomb_fish() {
//         let settings = Settings::default();
//         let mut detector = SonarDetector::new(&settings);

//         let img = image::open("assets\\sonar_bomb_fish.png")
//             .expect("Can't load image!")
//             .to_rgb8();

//         let words = detector.get_text_from_frame(&img);

//         assert!(
//             words.contains(&"Fish".to_owned()),
//             "Not founded \"fish\" on image but found {:?}.",
//             words
//         );
//     }

//     #[test]
//     fn find_base_text() {
//         let settings = Settings::default();
//         let mut detector = SonarDetector::new(&settings);

//         let img = image::open("assets\\base_text.png")
//             .expect("Can't load image!")
//             .to_rgb8();

//         let words = detector.get_text_from_frame(&img);

//         assert!(
//             words.contains(&"common".to_owned()),
//             "Not founded \"common\" on image but found {:?}.",
//             words
//         );
//     }
// }
