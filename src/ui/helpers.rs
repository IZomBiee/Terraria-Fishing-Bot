use crate::bot::types::DetectionMethod;
use crate::ui::App;
use device_query::Keycode;
use eframe::egui;
use std::str::FromStr;

impl App {
    pub(crate) fn update_preview(&mut self, ctx: &egui::Context, new_frame: &mut image::RgbaImage) {
        let size = [new_frame.width() as usize, new_frame.height() as usize];
        let pixels = new_frame.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        if let Some(handle) = &mut self.texture {
            handle.set(color_image, egui::TextureOptions::LINEAR);
        } else {
            self.texture =
                Some(ctx.load_texture("screen-capture", color_image, Default::default()));
        }
    }

    pub(crate) fn sync_local_settings(&mut self) {
        if self.local_settings != *self.settings.read().unwrap() {
            match self.local_settings.bot.detection_method {
                DetectionMethod::Movemap => {
                    self.local_settings.capture.fps = self.local_settings.movemap.fps;
                    self.local_settings.capture.margin = self.local_settings.movemap.margin;
                }
                DetectionMethod::Ocr => {
                    self.local_settings.capture.fps = self.local_settings.ocr.fps;
                    self.local_settings.capture.margin = self.local_settings.ocr.margin;
                }
                DetectionMethod::Audio => {}
                DetectionMethod::YOLO => {}
            }

            *self.settings.write().unwrap() = self.local_settings.clone();
        }
    }

    pub fn keycodes_to_string(keycode_vector: &[Keycode]) -> String {
        if keycode_vector.is_empty() {
            return "None".to_string();
        }
        keycode_vector
            .iter()
            .map(|keycode| keycode.to_string())
            .collect::<Vec<String>>()
            .join("+")
    }

    pub fn string_to_keycodes(keycode_string: &str) -> Vec<Keycode> {
        if keycode_string.is_empty() {
            Vec::<Keycode>::new()
        } else {
            keycode_string
                .split('+')
                .filter_map(|keycode| Keycode::from_str(keycode.trim()).ok())
                .collect()
        }
    }
}
