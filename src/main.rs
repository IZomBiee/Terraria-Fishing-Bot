use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use terraria_fishing_bot::bot::{Bot, BotCommand};
use terraria_fishing_bot::controller::Controller;
use terraria_fishing_bot::cursor_capturer::CursorCapturer;
use terraria_fishing_bot::settings::Settings;
use terraria_fishing_bot::ui;

fn main() {
    let settings = Arc::new(Mutex::new(Settings::load_from_file("settings.json")));
    let shared_frame = Arc::new(Mutex::new(None));

    let settings_for_thread = Arc::clone(&settings);
    let frame_for_thread = Arc::clone(&shared_frame);
    std::thread::spawn(move || {
        let mut capturer = CursorCapturer::new(settings_for_thread, frame_for_thread);
        capturer.run();
    });

    let controller = Controller::new(Arc::clone(&settings));

    let is_running = Arc::new(AtomicBool::new(false));

    let bot_frame = shared_frame.clone();
    let mut bot = Bot::new(
        bot_frame,
        Arc::clone(&is_running),
        Arc::clone(&settings),
        controller,
        None,
    );
    std::thread::spawn(move || bot.run());

    let _ = ui::run(Arc::clone(&settings), Arc::clone(&shared_frame), is_running);
}
