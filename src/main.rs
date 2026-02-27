use device_query::{DeviceQuery, DeviceState, Keycode};

use show_image::{ImageView, create_window};
use terraria_fishing_bot::bot::Bot;
use terraria_fishing_bot::controller::Controller;
use terraria_fishing_bot::cursor_capturer::CursorCapturer;
use terraria_fishing_bot::settings::Settings;

#[show_image::main]
fn main() {
    let window = create_window("Terraria Preview", Default::default()).unwrap();

    let settings = Settings::load_from_file("settings.json");
    let mut capturer = CursorCapturer::new(&settings);
    let controller = Controller::new(&settings);

    let mut bot = Bot::new(&settings, controller, None);

    let device_state = DeviceState::new();

    println!("Press Q to start, R to stop, P to exit.");
    loop {
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::Q) {
            bot.start();
        } else if keys.contains(&Keycode::R) {
            bot.stop();
        } else if keys.contains(&Keycode::P) {
            settings.save_to_file("settings.json");
            std::process::exit(0);
        }

        let Some(mut current_frame) = capturer.get_frame() else {
            continue;
        };

        bot.update(&current_frame);

        bot.draw_detection_gap(&mut current_frame);

        let view = ImageView::new(
            show_image::ImageInfo::rgba8(current_frame.width(), current_frame.height()),
            &current_frame,
        );

        window.set_image("Terraria Preview", view).unwrap();
    }
}
