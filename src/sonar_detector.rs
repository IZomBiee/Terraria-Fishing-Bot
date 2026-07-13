use image::RgbImage;
use oar_ocr::domain::tasks::TextDetectionConfig;
use oar_ocr::prelude::*;
use regex::Regex;
use strsim::levenshtein;

pub struct SonarDetector {
    engine: OAROCR,
}

impl Default for SonarDetector {
    fn default() -> Self {
        let engine = OAROCRBuilder::new(
            "assets/PP-OCRv6_small_det.onnx",
            "assets/PP-OCRv6_small_rec.onnx",
            "assets/ppocr_keys.txt",
        )
        .text_detection_config(TextDetectionConfig {
            score_threshold: 0.2,
            box_threshold: 0.45,
            unclip_ratio: 1.4,
            ..Default::default()
        })
        .build()
        .expect("Failed to initialize OcrEngine via ort");

        SonarDetector { engine }
    }
}

impl SonarDetector {
    fn clean_ocr_text(input: &str) -> String {
        let re = Regex::new(r"[^a-zA-Z0-9\s.,!?-]").unwrap();
        re.replace_all(input, " ").to_string()
    }

    pub fn get_strings_from_frame(&mut self, frame: RgbImage) -> String {
        let results = self.engine.predict(vec![frame]).unwrap_or_default();

        if let Some(frame_result) = results.into_iter().next() {
            frame_result
                .text_regions
                .iter()
                .filter_map(|region| region.text_with_confidence()) // Grabs (String, f32)
                .map(|(text, _confidence)| SonarDetector::clean_ocr_text(&text).trim().to_owned())
                .filter(|string| !string.is_empty())
                .collect()
        } else {
            String::new()
        }
    }

    pub fn is_needed_string(needed_string: &str, string: &str, thresh: usize) -> bool {
        let distance = levenshtein(needed_string, string);
        distance <= thresh
    }
}

mod tests {
    use super::*;

    fn find(word: &str, img_path: &str) {
        let mut detector = SonarDetector::default();
        let img = image::open(img_path).expect("Can't load image!").to_rgb8();
        let word1 = detector.get_strings_from_frame(img);

        assert!(
            SonarDetector::is_needed_string(word, &word1, 2),
            "Not founded \"{}\" on image but found \"{}\".",
            word,
            word1
        );
    }

    #[test]
    fn find_bass() {
        find("bass", "assets\\sonar_bass.png");
    }

    #[test]
    fn find_bomb_fish() {
        find("bomb fish", "assets\\sonar_bomb_fish.png");
    }

    #[test]
    fn find_base_text() {
        find("The most common", "assets\\base_text.png");
    }
}
