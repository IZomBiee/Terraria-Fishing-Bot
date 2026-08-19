use image::buffer::ConvertBuffer;
use image::RgbaImage;
use oar_ocr::domain::tasks::TextDetectionConfig;
use oar_ocr::prelude::*;
use regex::Regex;
use strsim::levenshtein;

static DET_MODEL_BYTES: &[u8] = include_bytes!("../../assets/PP-OCRv6_small_det.onnx");
static REC_MODEL_BYTES: &[u8] = include_bytes!("../../assets/PP-OCRv6_small_rec.onnx");
static KEYS_STR: &str = include_str!("../../assets/ppocr_keys.txt");

pub struct Ocr {
    engine: OAROCR,
    pub last_detected_text: Option<String>,
}

impl Default for Ocr {
    fn default() -> Self {
        let engine = OAROCRBuilder::new(
            DET_MODEL_BYTES,
            REC_MODEL_BYTES,
            "",
        )
        .character_dict_content(KEYS_STR)
        .text_detection_config(TextDetectionConfig {
            score_threshold: 0.2,
            box_threshold: 0.45,
            unclip_ratio: 1.4,
            ..Default::default()
        })
        .build()
        .expect("Failed to initialize OcrEngine via ort");

        Ocr { engine, last_detected_text: None}
    }
}

impl Ocr {
    fn clean_ocr_text(input: &str) -> String {
        let re = Regex::new(r"[^a-zA-Z0-9\s.,!?-]").unwrap();
        re.replace_all(input, " ").to_string()
    }

    pub fn update(&mut self, frame: RgbaImage) {
        let frame = frame.convert();

        let results = self.engine.predict(vec![frame]).unwrap_or_default();

        if let Some(frame_result) = results.into_iter().next() {
            self.last_detected_text = Some(frame_result
                .text_regions
                .iter()
                .filter_map(|region| region.text_with_confidence())
                .map(|(text, _confidence)| Ocr::clean_ocr_text(&text).trim().to_owned())
                .filter(|string| !string.is_empty())
                .collect());
        } else {
            self.last_detected_text = None
        }
    }

    pub fn is_needed_string(&self, needed_string: &str, thresh: usize) -> bool {
        let needed_strings: Vec<&str> = needed_string.split(",").collect();

        let Some(ref string) = self.last_detected_text else {
            return false;
        };

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