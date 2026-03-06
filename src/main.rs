// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod bot;
pub mod controller;
pub mod cursor_capturer;
pub mod opencv;
pub mod settings;
pub mod sonar_detector;
pub mod ui;
pub mod ui_terminal;

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::bot::{BotSended, BotState};

fn main() {
    let terminal = Arc::new(Mutex::new(ui_terminal::UiTerminal::new(100)));

    let settings = Arc::new(Mutex::new(settings::Settings::load_from_file(
        "settings.json",
        &mut terminal.lock().expect("Mutex poison"),
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

    let sonar_detector = sonar_detector::SonarDetector::default();

    let (gui_tx, bot_rx) = mpsc::channel::<bot::BotCommand>();
    let (bot_tx, gui_rx) = mpsc::channel::<BotSended>();

    let shared_state = Arc::new(Mutex::new(BotState::Idle));

    let bot_frame = shared_frame.clone();
    let mut bot = bot::Bot::new(
        bot_rx,
        bot_tx,
        Arc::clone(&terminal),
        Arc::clone(&shared_state),
        bot_frame,
        Arc::clone(&settings),
        controller,
        sonar_detector,
    );
    std::thread::spawn(move || bot.run());

    let _ = ui::run(
        gui_tx,
        gui_rx,
        Arc::clone(&settings),
        Arc::clone(&terminal),
        Arc::clone(&shared_frame),
        Arc::clone(&shared_state),
    );
}
