use device_query::{DeviceQuery, DeviceState, Keycode};
use eframe::egui::Vec2;
use eframe::{egui, egui::panel::*};
use log::info;
use std::str::FromStr;
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
    SetFPS(f32)
}

struct App {
    tx: Sender<BotCommand>,
    rx: Receiver<UiSended>,
    log_rx: Receiver<String>,
    settings: Arc<RwLock<Settings>>,
    terminal: UiTerminal,
    texture: Option<egui::TextureHandle>,
    shared_frame: SharedFrame,
    force_draw_frame: SharedFrame,
    last_toggle_key_time: Option<Instant>,
    last_liquid_gap: Option<(u32, u32)>,
    last_state: BotState,
    local_settings: Settings,
    fps: u64,
    bot_toggle_hotkey_buffer: Vec<Keycode>,
    is_choose_hotkey: bool,
}

pub fn run(
    tx: Sender<bot::BotCommand>,
    rx: Receiver<UiSended>,
    log_rx: Receiver<String>,
    settings: Arc<RwLock<Settings>>,
    terminal: UiTerminal,
    shared_frame: SharedFrame,
    force_draw_frame: SharedFrame,
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
                force_draw_frame,
                last_toggle_key_time: None,
                last_liquid_gap: None,
                last_state: BotState::Idle,
                fps: 0,
                bot_toggle_hotkey_buffer: Vec::new(),
                is_choose_hotkey: false
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
                    egui::Slider::new(&mut settings.liquid_detection_delay_ms, 500..=5000)
                        .text("Liquid Detection Delay (ms)"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.detection_gap_size_precents, 10..=100)
                        .text("Detection Gap Size (%)"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.calibration_time_ms, 3000..=10000)
                        .text("Calibration Time (ms)"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.calibration_multiplier_precents, 30..=100)
                        .text("Calibration Multiplier (%)"),
                );

                ui.horizontal(|ui| {
                    ui.label("Draw Mask");
                    ui.checkbox(&mut self.local_settings.movemap.draw_mask, "")
                });
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
        ui.add(
            egui::Slider::new(
                &mut self.local_settings.bot.catching_delay_millis,
                100..=1000,
            )
            .text("Catching Delay"),
        );
        ui.horizontal(|ui| {
            ui.label("Use Potions");
            ui.checkbox(&mut self.local_settings.bot.use_potions, "");
        });

        self.hotkey_selector(ui);
    }

    fn information_contents(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("State: {}", self.last_state));
        ui.label(format!("FPS: {}", self.fps));
        
        match self.last_state {
            BotState::Idle => {
                if ui.button("Enable bot").on_hover_text(
                    format!("Hotkey: {}", self.local_settings.bot.bot_toggle_hotkey.clone().unwrap_or("None".to_string()))
                ).clicked() {
                    let _ = self.tx.send(BotCommand::Start);
                }
            }
            _ => {
                if ui
                    .button("Disable bot")
                    .on_hover_text(format!("Hotkey: {}",
                    App::keycodes_to_string(&self.bot_toggle_hotkey_buffer)))
                    .clicked()
                {
                    let _ = self.tx.send(BotCommand::Stop);
                }
            }
        }
    }

    fn hotkey_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Hotkey: ");
            if self.is_choose_hotkey {
                let device_state = DeviceState::new();
                self.bot_toggle_hotkey_buffer = device_state.get_keys();
                
                if self.bot_toggle_hotkey_buffer.contains(&Keycode::Escape) {
                    self.is_choose_hotkey = false;
                    self.last_toggle_key_time = Some(Instant::now());
                } else if self.bot_toggle_hotkey_buffer.iter().any(|keycode| {keycode.to_string().len()==1}) {
                    self.local_settings.bot.bot_toggle_hotkey = Some(App::keycodes_to_string(&self.bot_toggle_hotkey_buffer));
                    self.is_choose_hotkey = false;
                    self.last_toggle_key_time = Some(Instant::now());
                }

                if self.bot_toggle_hotkey_buffer.len() == 0 {
                    let _ = ui.button("...");
                } else {
                    let _ = ui.button(App::keycodes_to_string(&self.bot_toggle_hotkey_buffer));
                }

            } else {
                if ui.button(self.local_settings.bot.bot_toggle_hotkey.clone().unwrap_or("None".to_owned())).clicked() {
                    self.is_choose_hotkey = true;
                }
            }
        });
    }

    pub fn keycodes_to_string(keycode_vector: &Vec<Keycode>) -> String {
        if keycode_vector.is_empty() {
            return "None".to_string();
        }
        keycode_vector.iter().map(|keycode| keycode.to_string()).collect::<Vec<String>>().join("+")
    }

    pub fn string_to_keycodes(keycode_string: &str) -> Vec<Keycode> {
        if keycode_string.is_empty() {
            Vec::<Keycode>::new()
        } else {
            keycode_string.split("+").map(| keycode| Keycode::from_str(keycode).unwrap()).collect::<Vec<Keycode>>()
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
                UiSended::DetectedItems(items) => {
                    info!("Detected words {}", items.join(" "));
                },
                UiSended::ChangeState(state) => self.last_state = state,
                UiSended::SetFPS(fps) => {
                    self.fps = fps as u64
                }
            }
        }

        while let Ok(recived) = self.log_rx.try_recv() {
            self.terminal.print(&recived);
        }

        let device_state = DeviceState::new();
        let keys = device_state.get_keys();
        if let Some(hotkey) = &self.local_settings.bot.bot_toggle_hotkey {
            let needed_keycodes = App::string_to_keycodes(hotkey);
            if keys.eq(&needed_keycodes) {
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
        }

        let maybe_img = self
            .force_draw_frame
            .lock()
            .ok()
            .and_then(|guard| guard.clone());

        if let Some(mut rgba_img) = maybe_img {
            self.update_preview(ctx, &mut rgba_img);
        } else {
            let maybe_img = self
                .shared_frame
                .lock()
                .ok()
                .and_then(|guard| guard.clone());
            if let Some(mut rgba_img) = maybe_img {
                self.update_preview(ctx, &mut rgba_img);
            }
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
