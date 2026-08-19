use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;
use image::{DynamicImage, Rgba, RgbaImage};
use ultralytics_inference::{InferenceConfig, YOLOModel};
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;

static MODEL_BYTES: &[u8] = include_bytes!("../../assets/yolo_model.onnx");

#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub confidence: f32,
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32
}

impl Detection { 
    pub fn draw(&self, image: &mut RgbaImage) {
        let width = self.x1 as i32 - self.x0 as i32;
        let height = self.y1 as i32 - self.y0 as i32;

        if width <= 0 || height <= 0 {
            return;
        }
        let rect = Rect::at(self.x0 as i32, self.y0 as i32).of_size(width as u32, height as u32);
        let color = Rgba([0, 0, 255, 255]);
        draw_hollow_rect_mut(image, rect, color);
    }
}

pub struct OnnxYolo {
    model: YOLOModel,
    pub previous_detection: Option<Detection>,
    pub last_detection: Option<Detection>
}

impl Default for OnnxYolo {
    fn default() -> Self {
        let config = InferenceConfig::new()
            .with_confidence(0.15)
            .with_iou(0.15)
            .with_imgsz(640, 640);

        let temp_path = std::env::temp_dir().join("terraria_yolo_model.onnx");
        if !temp_path.exists() {
            let mut file = File::create(&temp_path).expect("Failed to create temp model file");
            file.write_all(MODEL_BYTES).expect("Failed to write model bytes");
        }

        let model = YOLOModel::load_with_config(&temp_path, config.clone())
            .expect("Failed to load ONNX model");

        Self {
            model,
            previous_detection: None,
            last_detection: None,
        }
    }
}

impl OnnxYolo {
    pub fn update(&mut self, image: &RgbaImage) {
        let dynamic_img = DynamicImage::ImageRgba8(image.clone());

        self.previous_detection = self.last_detection.clone();

        let Ok(results) = self.model.predict_image(&dynamic_img, String::new()) else {
            self.last_detection = None;
            return;
        };

        let Some(first_result) = results.first() else {
            self.last_detection = None;
            return;
        };

        let Some(most_confident_box) = first_result.boxes.iter().max_by(|a, b| {
            let conf_a = a.conf().first().copied().unwrap_or(0.0);
            let conf_b = b.conf().first().copied().unwrap_or(0.0);
            conf_a.partial_cmp(&conf_b).unwrap_or(Ordering::Equal)
        }) else {
            self.last_detection = None;
            return;
        };

        let xyxy = most_confident_box.xyxy();
        let x0 = xyxy[[0, 0]].max(0.0).round() as u32;
        let y0 = xyxy[[0, 1]].max(0.0).round() as u32;
        let x1 = xyxy[[0, 2]].max(0.0).round() as u32;
        let y1 = xyxy[[0, 3]].max(0.0).round() as u32;

        let confidence = most_confident_box.conf().first().copied().unwrap_or(0.0);

        let detection = Detection { confidence, x0, y0, x1, y1};

        self.last_detection = Some(detection.clone());
    }
}