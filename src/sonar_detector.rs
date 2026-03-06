use image::{DynamicImage, RgbImage};
use ocr_rs::OcrEngine;
use regex::Regex;
use strsim::levenshtein;

pub struct SonarDetector {
    engine: OcrEngine,
}

impl Default for SonarDetector {
    fn default() -> Self {
        let engine = OcrEngine::new(
            "assets\\PP-OCRv5_mobile_det_fp16.mnn",
            "assets\\en_PP-OCRv5_mobile_rec_infer.mnn",
            "assets/ppocr_keys.txt",
            None,
        )
        .expect("Failed to initialize OCR Engine");

        SonarDetector { engine }
    }
}

impl SonarDetector {
    fn clean_ocr_text(input: &str) -> String {
        let re = Regex::new(r"[^a-zA-Z0-9\s.,!?-]").unwrap();
        re.replace_all(input, " ").to_string()
    }

    pub fn get_strings_from_frame(&mut self, frame: RgbImage) -> Vec<String> {
        let results = self
            .engine
            .recognize(&DynamicImage::ImageRgb8(frame))
            .unwrap_or_default();

        results
            .iter()
            .map(|result| {
                SonarDetector::clean_ocr_text(&result.text)
                    .trim()
                    .to_owned()
            })
            .filter(|string| !string.is_empty())
            .collect()
    }

    pub fn is_needed_string(needed_string: &str, strings: Vec<String>, thresh: usize) -> bool {
        strings
            .iter()
            .map(|string| levenshtein(needed_string, string))
            .any(|distance| distance < thresh)
    }
}

mod tests {
    use super::*;

    fn find(word: &str, img_path: &str) {
        let mut detector = SonarDetector::default();
        let img = image::open(img_path).expect("Can't load image!").to_rgb8();
        let words = detector.get_strings_from_frame(img);

        assert!(
            SonarDetector::is_needed_string(word, words.clone(), 1),
            "Not founded \"{}\" on image but found {:?}.",
            word,
            words
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
        find("The most commond", "assets\\base_text.png");
    }
}
