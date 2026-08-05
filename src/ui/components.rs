use crate::bot::types::{BotCommand, BotState, DetectionMethod};
use crate::ui::App;
use device_query::{DeviceQuery, DeviceState, Keycode};
use eframe::egui;
use std::time::Instant;

impl App {
    pub(crate) fn detection_contents(&mut self, ui: &mut egui::Ui) {
        let detection_method = &mut self.local_settings.bot.detection_method;

        if self.last_state == BotState::Idle {
            ui.horizontal(|ui| {
                ui.label("Method");
                egui::ComboBox::from_id_salt("method_combobox")
                    .selected_text(format!("{:?}", detection_method))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(detection_method, DetectionMethod::Movemap, "MoveMap");
                        ui.selectable_value(detection_method, DetectionMethod::Ocr, "Ocr");
                        ui.selectable_value(detection_method, DetectionMethod::Audio, "Audio");
                    });
            });
        }

        match detection_method {
            DetectionMethod::Movemap => {
                let settings = &mut self.local_settings.movemap;

                ui.add(
                    egui::Slider::new(&mut settings.margin, 10..=300)
                        .text("Detection Area"),
                );
                ui.add(egui::Slider::new(&mut settings.fps, 5..=60).text("Framerate"));

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
            DetectionMethod::Ocr => {
                let settings = &mut self.local_settings.ocr;

                ui.add(
                    egui::Slider::new(&mut settings.margin, 100..=500)
                        .text("Detection Area"),
                );
                ui.add(egui::Slider::new(&mut settings.fps, 3..=20).text("Framerate"));

                ui.label("Detection Words");
                egui::TextEdit::multiline(&mut settings.sonar_detection_words)
                    .hint_text("Write all items you want to catch and separate by comma(,). Like create, bomb.")
                    .show(ui);
                ui.add(
                    egui::Slider::new(&mut settings.sonar_detection_threshold, 0..=10)
                        .text("Threshold"),
                );
            }
            DetectionMethod::Audio => {
                let settings = &mut self.local_settings.audio;
                ui.add(
                    egui::Slider::new(&mut settings.min_confidence, 0..=100).text("Min Confidence"),
                );
            }
        };
    }

    pub(crate) fn general_contents(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn information_contents(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("State: {}", self.last_state));
        ui.label(format!("Bot FPS: {:.1}", self.bot_fps));
        ui.label(format!("Capture FPS: {:.1}", self.capture_fps));

        match self.last_state {
            BotState::Idle => {
                if ui
                    .button("Enable bot")
                    .on_hover_text(format!(
                        "Hotkey: {}",
                        self.local_settings
                            .bot
                            .bot_toggle_hotkey
                            .clone()
                            .unwrap_or("None".to_string())
                    ))
                    .clicked()
                {
                    let _ = self.tx.send(BotCommand::Start);
                }
            }
            _ => {
                if ui
                    .button("Disable bot")
                    .on_hover_text(format!(
                        "Hotkey: {}",
                        App::keycodes_to_string(&self.bot_toggle_hotkey_buffer)
                    ))
                    .clicked()
                {
                    let _ = self.tx.send(BotCommand::Stop);
                }
            }
        }
    }

    pub(crate) fn hotkey_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Hotkey: ");
            if self.is_choose_hotkey {
                let device_state = DeviceState::new();
                self.bot_toggle_hotkey_buffer = device_state.get_keys();

                if self.bot_toggle_hotkey_buffer.contains(&Keycode::Escape) {
                    self.is_choose_hotkey = false;
                    self.last_toggle_key_time = Some(Instant::now());
                } else if self
                    .bot_toggle_hotkey_buffer
                    .iter()
                    .any(|keycode| keycode.to_string().len() == 1)
                {
                    self.local_settings.bot.bot_toggle_hotkey =
                        Some(App::keycodes_to_string(&self.bot_toggle_hotkey_buffer));
                    self.is_choose_hotkey = false;
                    self.last_toggle_key_time = Some(Instant::now());
                }

                if self.bot_toggle_hotkey_buffer.is_empty() {
                    let _ = ui.button("...");
                } else {
                    let _ = ui.button(App::keycodes_to_string(&self.bot_toggle_hotkey_buffer));
                }
            } else {
                if ui
                    .button(
                        self.local_settings
                            .bot
                            .bot_toggle_hotkey
                            .clone()
                            .unwrap_or("None".to_owned()),
                    )
                    .clicked()
                {
                    self.is_choose_hotkey = true;
                }
            }
        });
    }
}
