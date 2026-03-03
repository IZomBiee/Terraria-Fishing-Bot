#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod bot;
pub mod controller;
pub mod cursor_capturer;
pub mod opencv;
pub mod settings;
pub mod sonar_detector;
pub mod ui;

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::bot::BotState;

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

    let sonar_detector = sonar_detector::SonarDetector::new(Arc::clone(&settings));

    let (gui_tx, bot_rx) = mpsc::channel::<bot::BotCommand>();
    // let (bot_tx, gui_rx) = mpsc::channel::<BotData>();

    let shared_state = Arc::new(Mutex::new(BotState::Idle));

    let bot_frame = shared_frame.clone();
    let mut bot = bot::Bot::new(
        bot_rx,
        Arc::clone(&shared_state),
        bot_frame,
        Arc::clone(&settings),
        controller,
        sonar_detector,
    );
    std::thread::spawn(move || bot.run());

    let _ = ui::run(
        gui_tx,
        Arc::clone(&settings),
        Arc::clone(&shared_frame),
        Arc::clone(&shared_state),
    );
}
