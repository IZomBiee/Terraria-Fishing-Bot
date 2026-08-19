use device_query::Keycode;
use eframe::egui;
use std::sync::{
    Arc, RwLock,
    mpsc::{Receiver, Sender},
};
use std::time::Instant;

pub mod terminal;
pub use self::terminal::Terminal;

mod app_impl;
mod components;
mod helpers;

use crate::bot::types::{BotCommand, BotState};
use crate::cursor_capture_server::SharedFrame;
use crate::settings::Settings;

pub enum UiSended {
    ChangeState(BotState),
    SetBotFps(f32),
    SetCaptureFps(f32),
}

struct App {
    tx: Sender<BotCommand>,
    rx: Receiver<UiSended>,
    log_rx: Receiver<String>,
    settings: Arc<RwLock<Settings>>,
    terminal: Terminal,
    texture: Option<egui::TextureHandle>,
    draw_frame: SharedFrame,
    last_toggle_key_time: Option<Instant>,
    last_state: BotState,
    local_settings: Settings,
    bot_fps: f32,
    capture_fps: f32,
    bot_toggle_hotkey_buffer: Vec<Keycode>,
    is_choose_hotkey: bool,
}

pub fn run(
    tx: Sender<BotCommand>,
    rx: Receiver<UiSended>,
    log_rx: Receiver<String>,
    settings: Arc<RwLock<Settings>>,
    terminal: Terminal,
    draw_frame: SharedFrame,
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
                draw_frame,
                last_toggle_key_time: None,
                last_state: BotState::Idle,
                bot_fps: 0.0,
                capture_fps: 0.0,
                bot_toggle_hotkey_buffer: Vec::new(),
                is_choose_hotkey: false,
            }))
        }),
    )
}
