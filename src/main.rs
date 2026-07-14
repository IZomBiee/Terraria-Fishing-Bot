#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod audio_server;
pub mod bot;
pub mod controller;
pub mod cursor_capture_server;
pub mod logger;
pub mod onnx_audio_detector;
pub mod opencv;
pub mod settings;
pub mod sonar_detector;
pub mod ui;
pub mod ui_terminal;

use eframe::egui;
use image::RgbaImage;
use logger::Logger;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

fn main() {
    let (log_tx, log_rx) = mpsc::channel();

    let logger = Box::new(Logger {
        tx: Mutex::new(log_tx),
        start_time: Instant::now(),
    });

    log::set_logger(Box::leak(logger)).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    let terminal = ui_terminal::UiTerminal::new(100);

    let settings = Arc::new(RwLock::new(settings::Settings::load_from_file(
        "settings.json",
    )));
    let shared_frame = Arc::new(Mutex::new(None));
    let force_draw_frame: Arc<Mutex<Option<RgbaImage>>> = Arc::new(Mutex::new(None));

    let force_draw_bot_frame = Arc::clone(&force_draw_frame);
    let force_draw_ui_frame = Arc::clone(&force_draw_frame);

    let settings_for_thread = Arc::clone(&settings);
    let frame_for_thread = Arc::clone(&shared_frame);

    let (ctx_tx_capturer, ctx_rx_capturer) = std::sync::mpsc::channel::<egui::Context>();

    std::thread::spawn(move || {
        if let Ok(egui_ctx) = ctx_rx_capturer.recv() {
            let mut capturer = cursor_capture_server::CursorCaptureServer::new(
                settings_for_thread,
                egui_ctx,
                frame_for_thread,
            );
            capturer.run();
        }
    });

    let sonar_detector = sonar_detector::SonarDetector::default();

    let (gui_tx, bot_rx) = mpsc::channel::<bot::BotCommand>();
    let (bot_tx, gui_rx) = mpsc::channel::<ui::UiSended>();

    let bot_frame = shared_frame.clone();

    let settings_for_bot = Arc::clone(&settings);

    let (ctx_tx_bot, ctx_rx_bot) = std::sync::mpsc::channel::<egui::Context>();

    std::thread::spawn(move || {
        if let Ok(egui_ctx) = ctx_rx_bot.recv() {
            let mut bot = bot::Bot::new(
                bot_rx,
                bot_tx,
                bot_frame,
                force_draw_bot_frame,
                settings_for_bot,
                sonar_detector,
                egui_ctx,
            );
            bot.run();
        }
    });

    let _ = ui::run(
        gui_tx,
        gui_rx,
        log_rx,
        Arc::clone(&settings),
        terminal,
        Arc::clone(&shared_frame),
        force_draw_ui_frame,
        ctx_tx_capturer,
        ctx_tx_bot,
    );
}
