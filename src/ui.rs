use std::sync::{Arc, RwLock, mpsc::{Receiver, Sender}};
use std::time::Instant;
use eframe::egui;
use device_query::Keycode;

pub mod terminal;
pub use self::terminal::Terminal;

mod app_impl;
mod components;
mod helpers;

use crate::bot::types::{DetectionMethod, BotCommand, BotState};
use crate::cursor_capture_server::SharedFrame;
use crate::settings::Settings;

pub enum UiSended {
    LiquidGap(Option<(u32, u32)>),
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
    shared_frame: SharedFrame,
    force_draw_frame: SharedFrame,
    last_toggle_key_time: Option<Instant>,
    last_liquid_gap: Option<(u32, u32)>,
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
    shared_frame: SharedFrame,
    force_draw_frame: SharedFrame,
    ctx_tx_capturer: Sender<egui::Context>,
    ctx_tx_bot: Sender<egui::Context>,
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

            let _ = ctx_tx_capturer.send(cc.egui_ctx.clone());
            let _ = ctx_tx_bot.send(cc.egui_ctx.clone());

            Ok(Box::new(App {
                tx, rx, log_rx, settings, local_settings, terminal,
                texture: None, shared_frame, force_draw_frame,
                last_toggle_key_time: None, last_liquid_gap: None,
                last_state: BotState::Idle, bot_fps: 0.0, capture_fps: 0.0,
                bot_toggle_hotkey_buffer: Vec::new(), is_choose_hotkey: false,
            }))
        }),
    )
}