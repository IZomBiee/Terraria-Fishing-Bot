use device_query::{DeviceQuery, DeviceState, Keycode};
use eframe::egui::Vec2;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::{egui, egui::panel::*};

use crate::bot::DetectionMethod;
use crate::cursor_capturer::SharedFrame;
use crate::settings::Settings;

pub fn run(
    settings: Arc<Mutex<Settings>>,
    shared_frame: SharedFrame,
    is_running: Arc<AtomicBool>,
) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([660.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Terraria-Fishing-Bot",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App {
                settings,
                texture: None,
                is_running,
                shared_frame,
            }))
        }),
    )
}
struct App {
    settings: Arc<Mutex<Settings>>,
    texture: Option<egui::TextureHandle>,
    is_running: Arc<AtomicBool>,
    shared_frame: SharedFrame,
}

impl App {
    fn update_preview(&mut self, ctx: &egui::Context, new_frame: &image::RgbaImage) {
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

    fn detection_contents(&mut self, ui: &mut egui::Ui) {
        let Ok(mut settings) = self.settings.lock() else {
            return;
        };

        egui::Grid::new("detection_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Method");
                egui::ComboBox::from_id_salt("method_combobox")
                    .selected_text(format!("{:?}", settings.detection_method))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut settings.detection_method,
                            DetectionMethod::MoveMap,
                            "MoveMap",
                        );
                        ui.selectable_value(
                            &mut settings.detection_method,
                            DetectionMethod::Yolo,
                            "YOLO",
                        );
                        ui.selectable_value(
                            &mut settings.detection_method,
                            DetectionMethod::Sonar,
                            "Sonar",
                        );
                    });
                ui.end_row();
                match settings.detection_method {
                    DetectionMethod::MoveMap => {
                        ui.label("Liquid Offset");
                        ui.add(egui::Slider::new(&mut settings.liquid_offset, -20..=20));
                        ui.end_row();
                        ui.label("Liquid Detection Delay");
                        ui.add(egui::Slider::new(
                            &mut settings.liquid_detection_delay_millis,
                            500..=5000,
                        ));
                        ui.end_row();
                        ui.label("Noise Detection Delay");
                        ui.add(egui::Slider::new(
                            &mut settings.noises_delay_millis,
                            500..=5000,
                        ));
                        ui.end_row();

                        ui.label("Detection Threshold");
                        ui.add(egui::Slider::new(
                            &mut settings.detection_threshold,
                            50..=500,
                        ));
                        ui.end_row();

                        ui.label("Detection Gap Size");
                        ui.add(egui::Slider::new(
                            &mut settings.detection_gap_size,
                            10..=100,
                        ));
                    }
                    DetectionMethod::Sonar => {}
                    _ => {}
                };
            });
    }

    fn general_contents(&mut self, ui: &mut egui::Ui) {
        let Ok(mut settings) = self.settings.lock() else {
            return;
        };

        egui::Grid::new("general_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Detection Area");
                ui.add(egui::Slider::new(&mut settings.margin, 10..=300));
                ui.end_row();
                ui.label("Framerate");
                ui.add(egui::Slider::new(&mut settings.fps, 5..=60));
                ui.end_row();
                ui.label("Casting Delay");
                ui.add(egui::Slider::new(
                    &mut settings.casting_delay_millis,
                    300..=1500,
                ));
                ui.end_row();
                ui.label("Use Potions");
                ui.checkbox(&mut settings.use_potions, "");
            });
    }

    fn information_contents(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("information_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Status:");
                ui.label("None");
                ui.end_row();
            });
        if self.is_running.load(Ordering::Relaxed)
            && ui
                .button("Disable bot")
                .on_hover_text("Hotkey: P")
                .clicked()
        {
            self.is_running.store(false, Ordering::Relaxed);
        } else if !self.is_running.load(Ordering::Relaxed)
            && ui.button("Enable bot").on_hover_text("Hotkey: P").clicked()
        {
            self.is_running.store(true, Ordering::Relaxed);
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        println!("Saving data before exit...");
        if let Ok(settings) = self.settings.lock() {
            settings.save_to_file("settings.json");
        };
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let device_state = DeviceState::new();
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::P) {
            self.is_running
                .store(!self.is_running.load(Ordering::Relaxed), Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(300));
        };

        let maybe_img = self
            .shared_frame
            .lock()
            .ok()
            .and_then(|guard| guard.clone()); // Clones the Option<ImageBuffer>

        if let Some(rgba_img) = maybe_img {
            self.update_preview(ctx, &rgba_img);
        }

        SidePanel::left("left_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("General")
                            .color(egui::Color32::WHITE)
                            .size(18f32),
                    );
                });

                self.general_contents(ui);

                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Detection")
                            .color(egui::Color32::WHITE)
                            .size(18f32),
                    );
                });

                self.detection_contents(ui);

                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Info")
                            .color(egui::Color32::WHITE)
                            .size(18f32),
                    );
                });

                self.information_contents(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.texture {
                ui.add(
                    egui::Image::from_texture(texture).fit_to_exact_size(Vec2::new(385.0, 385.0)),
                );
            } else {
                ui.label("Waiting for screen capture...");
            }
        });

        // ctx.request_repaint();
    }
}
