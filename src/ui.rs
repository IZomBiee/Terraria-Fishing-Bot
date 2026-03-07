use device_query::{DeviceQuery, DeviceState, Keycode};
use eframe::egui::Vec2;
use eframe::{egui, egui::panel::*};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock, mpsc::Sender};
use std::time::{Duration, Instant};

use crate::bot::{self, BotCommand, BotState, DetectionMethod};
use crate::cursor_capturer::SharedFrame;
use crate::settings::Settings;
use crate::ui_terminal::UiTerminal;

pub enum UiSended {
    LiquidGap(u32, u32),
    DetectedItems(Vec<String>),
    ChangeState(BotState),
}

struct App {
    tx: Sender<BotCommand>,
    rx: Receiver<UiSended>,
    log_rx: Receiver<String>,
    settings: Arc<RwLock<Settings>>,
    terminal: UiTerminal,
    texture: Option<egui::TextureHandle>,
    shared_frame: SharedFrame,
    last_toggle_key_time: Option<Instant>,
    last_liquid_gap: Option<(u32, u32)>,
    last_state: BotState,
    local_settings: Settings,
}

pub fn run(
    tx: Sender<bot::BotCommand>,
    rx: Receiver<UiSended>,
    log_rx: Receiver<String>,
    settings: Arc<RwLock<Settings>>,
    terminal: UiTerminal,
    shared_frame: SharedFrame,
) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Terraria-Fishing-Bot",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let local_settings = settings.read().unwrap().clone();
            Ok(Box::new(App {
                tx,
                rx,
                log_rx,
                settings,
                local_settings,
                terminal,
                texture: None,
                shared_frame,
                last_toggle_key_time: None,
                last_liquid_gap: None,
                last_state: BotState::Idle,
            }))
        }),
    )
}

impl App {
    fn update_preview(&mut self, ctx: &egui::Context, new_frame: &mut image::RgbaImage) {
        if let Some((y0, y1)) = self.last_liquid_gap
            && self.settings.read().unwrap().bot.detection_method == DetectionMethod::MoveMap
        {
            let (width, height) = new_frame.dimensions();

            for (y, color) in [(y0, [0, 0, 255, 255]), (y1, [255, 0, 0, 255])] {
                if y < height {
                    for x in 0..width {
                        new_frame.put_pixel(x, y, image::Rgba(color));
                    }
                }
            }
        }

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
        let detection_method = &mut self.local_settings.bot.detection_method;

        ui.horizontal(|ui| {
            ui.label("Method");
            egui::ComboBox::from_id_salt("method_combobox")
                .selected_text(format!("{:?}", detection_method))
                .show_ui(ui, |ui| {
                    ui.selectable_value(detection_method, DetectionMethod::MoveMap, "MoveMap");
                    ui.selectable_value(detection_method, DetectionMethod::Yolo, "YOLO");
                    ui.selectable_value(detection_method, DetectionMethod::Sonar, "Sonar");
                });
        });

        match detection_method {
            DetectionMethod::MoveMap => {
                let settings = &mut self.local_settings.movemap;
                ui.add(
                    egui::Slider::new(&mut settings.liquid_offset, -20..=20).text("Liquid Offset"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.liquid_detection_delay_millis, 500..=5000)
                        .text("Liquid Detection Delay"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.noises_delay_millis, 500..=5000)
                        .text("Noise Detection Delay"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.detection_threshold, 50..=500)
                        .text("Detection Threshold"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.detection_gap_size, 10..=100)
                        .text("Detection Gap Size"),
                );
            }
            DetectionMethod::Sonar => {
                let settings = &mut self.local_settings.sonar;

                ui.label("Detection Words");
                egui::TextEdit::multiline(&mut settings.sonar_detection_words)
                .hint_text("Write all items you want to catch and separate by comma(,). Like create, bomb.")
                .show(ui);

                ui.add(
                    egui::Slider::new(&mut settings.sonar_detection_threshold, 0..=10)
                        .text("Threshold"),
                );
            }
            _ => {}
        };
    }

    fn general_contents(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::Slider::new(&mut self.local_settings.capture.margin, 10..=300)
                .text("Detection Area"),
        );
        ui.add(egui::Slider::new(&mut self.local_settings.capture.fps, 5..=60).text("Framerate"));
        ui.add(
            egui::Slider::new(
                &mut self.local_settings.bot.casting_delay_millis,
                300..=1500,
            )
            .text("Casting Delay"),
        );
        ui.horizontal(|ui| {
            ui.label("Use Potions");
            ui.checkbox(&mut self.local_settings.bot.use_potions, "");
        });
    }

    fn information_contents(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("State: {:?}", self.last_state));

        match self.last_state {
            BotState::Idle => {
                if ui.button("Enable bot").on_hover_text("Hotkey: P").clicked() {
                    let _ = self.tx.send(BotCommand::Start);
                }
            }
            _ => {
                if ui
                    .button("Disable bot")
                    .on_hover_text("Hotkey: P")
                    .clicked()
                {
                    let _ = self.tx.send(BotCommand::Stop);
                }
            }
        }
    }

    fn sync_local_settings(&mut self) {
        if self.local_settings != *self.settings.read().unwrap() {
            *self.settings.write().unwrap() = self.local_settings.clone();
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.terminal.print("Saving data before exit...");
        self.settings.read().unwrap().save_to_file("settings.json");
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(recived) = self.rx.try_recv() {
            match recived {
                UiSended::LiquidGap(y0, y1) => self.last_liquid_gap = Some((y0, y1)),
                UiSended::DetectedItems(_items) => (),
                UiSended::ChangeState(state) => self.last_state = state,
            }
        }

        while let Ok(recived) = self.log_rx.try_recv() {
            self.terminal.print(&recived);
        }

        let device_state = DeviceState::new();
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::P) {
            if self.last_toggle_key_time.is_none()
                || self.last_toggle_key_time.unwrap().elapsed() > Duration::from_millis(100)
            {
                match self.last_state {
                    BotState::Idle => {
                        let _ = self.tx.send(BotCommand::Start);
                    }
                    _ => {
                        let _ = self.tx.send(BotCommand::Stop);
                    }
                }
            }
            self.last_toggle_key_time = Some(Instant::now());
        };

        let maybe_img = self
            .shared_frame
            .lock()
            .ok()
            .and_then(|guard| guard.clone());

        if let Some(mut rgba_img) = maybe_img {
            self.update_preview(ctx, &mut rgba_img);
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

        SidePanel::right("right_panel").show(ctx, |ui| {
            self.terminal.show(ui);
        });

        CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.texture {
                ui.add(
                    egui::Image::from_texture(texture)
                        .fit_to_exact_size(Vec2::new(ui.available_width(), ui.available_height())),
                );
            } else {
                ui.label("Waiting for screen capture...");
            }
        });

        self.sync_local_settings();

        ctx.request_repaint();
    }
}
