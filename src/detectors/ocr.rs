use image::{Pixel, RgbImage, RgbaImage};
use oar_ocr::domain::tasks::TextDetectionConfig;
use oar_ocr::prelude::*;
use regex::Regex;
use strsim::levenshtein;

pub struct Ocr {
    engine: OAROCR,
}

impl Default for Ocr {
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

        Ocr { engine }
    }
}

impl Ocr {
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
                .map(|(text, _confidence)| Ocr::clean_ocr_text(&text).trim().to_owned())
                .filter(|string| !string.is_empty())
                .collect()
        } else {
            String::new()
        }
    }

    pub fn is_needed_string(needed_string: &str, string: &str, thresh: usize) -> bool {
        let needed_strings: Vec<&str> = needed_string.split(",").collect();

        for needed_string in &needed_strings {
            let needed_string = needed_string.trim();
            if levenshtein(needed_string, string) < thresh {
                return true;
            }
        }

        let strings: Vec<&str> = string.split(" ").collect();
        for string in strings {
            let string = string.trim();
            for needed_string in &needed_strings {
                if levenshtein(needed_string, string) < thresh {
                    return true;
                }
            }
        }

        let strings: Vec<&str> = string.split(" ").collect();
        for string in strings {
            let string = string.trim();
            for needed_string in &needed_strings {
                let needed_string = needed_string.trim();
                for needed_word in needed_string.split(" ") {
                    let needed_word = needed_word.trim();
                    if levenshtein(needed_word, string) < thresh {
                        return true;
                    }
                }
            }
        }

        return false;
    }
}

pub fn rgba_2_rgb(rgba: &RgbaImage) -> RgbImage {
    let (width, height) = rgba.dimensions();
    let mut rgb = RgbImage::new(width, height);

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let channels = pixel.channels();
        rgb.put_pixel(x, y, image::Rgb([channels[0], channels[1], channels[2]]));
    }

    rgb
}
