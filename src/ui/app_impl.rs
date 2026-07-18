use crate::ui::{App, UiSended};
use crate::bot::types::{BotCommand, BotState};
use eframe::egui;
use eframe::egui::Vec2;
use eframe::egui::panel::{SidePanel, CentralPanel};
use device_query::{DeviceQuery, DeviceState};
use std::time::{Duration, Instant};

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.terminal.print("Saving data before exit...");
        self.settings.read().unwrap().save_to_file("settings.json");
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(recived) = self.rx.try_recv() {
            match recived {
                UiSended::LiquidGap(liquid_gap) => self.last_liquid_gap = liquid_gap,
                UiSended::ChangeState(state) => self.last_state = state,
                UiSended::SetBotFps(fps) => self.bot_fps = fps,
                UiSended::SetCaptureFps(fps) => self.capture_fps = fps,
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
                        BotState::Idle => { let _ = self.tx.send(BotCommand::Start); }
                        _ => { let _ = self.tx.send(BotCommand::Stop); }
                    }
                }
                self.last_toggle_key_time = Some(Instant::now());
            };
        }

        let maybe_img = self.force_draw_frame.lock().ok().and_then(|guard| guard.clone());

        if let Some(mut rgba_img) = maybe_img {
            self.update_preview(ctx, &mut rgba_img);
        } else {
            let maybe_img = self.shared_frame.lock().ok().and_then(|guard| guard.clone());
            if let Some(mut rgba_img) = maybe_img {
                self.update_preview(ctx, &mut rgba_img);
            }
        }

        SidePanel::left("left_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("General").color(egui::Color32::WHITE).size(18f32));
                });
                self.general_contents(ui);

                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Detection").color(egui::Color32::WHITE).size(18f32));
                });
                self.detection_contents(ui);

                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Info").color(egui::Color32::WHITE).size(18f32));
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
    }
}