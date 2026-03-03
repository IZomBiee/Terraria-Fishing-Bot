#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod bot;
pub mod controller;
pub mod cursor_capturer;
pub mod opencv;
pub mod settings;
pub mod sonar_detector;
pub mod ui;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

fn main() {
    let settings = Arc::new(Mutex::new(settings::Settings::load_from_file(
        "settings.json",
    )));
    let shared_frame = Arc::new(Mutex::new(None));

    let settings_for_thread = Arc::clone(&settings);
    let frame_for_thread = Arc::clone(&shared_frame);
    std::thread::spawn(move || {
        let mut capturer =
            cursor_capturer::CursorCapturer::new(settings_for_thread, frame_for_thread);
        capturer.run();
    });

    let controller = controller::Controller::new(Arc::clone(&settings));

    let is_running = Arc::new(AtomicBool::new(false));

    let bot_frame = shared_frame.clone();
    let mut bot = bot::Bot::new(
        bot_frame,
        Arc::clone(&is_running),
        Arc::clone(&settings),
        controller,
        None,
    );
    std::thread::spawn(move || bot.run());

    let _ = ui::run(Arc::clone(&settings), Arc::clone(&shared_frame), is_running);
}
