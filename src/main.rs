use device_query::{DeviceQuery, DeviceState, Keycode};
use show_image::{ImageView, create_window};
use terraria_fishing_bot::bot::Bot;
use terraria_fishing_bot::controller::Controller;
use terraria_fishing_bot::cursor_capturer::CursorCapturer;
use terraria_fishing_bot::settings::Settings;

#[show_image::main]
fn main() {
    let window_settings = show_image::WindowOptions {
        size: Some([500, 500]),
        ..Default::default()
    };

    let window_rgba = create_window("Terraria Preview - rgba", window_settings).unwrap();

    let settings = Settings::load_from_file("settings.json");
    let mut capturer = CursorCapturer::new(&settings);
    let controller = Controller::new(&settings);

    let mut bot = Bot::new(&settings, Some(controller));

    let device_state = DeviceState::new();

    println!("Press Q to start, R to stop.");
    loop {
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::Q) {
            bot.start();
        } else if keys.contains(&Keycode::R) {
            bot.stop();
        }

        let Some(mut current_frame) = capturer.get_frame() else {
            continue;
        };

        bot.update(&current_frame);

        bot.draw_detection_gap(&mut current_frame);

        let view_rgba = ImageView::new(
            show_image::ImageInfo::rgba8(current_frame.width(), current_frame.height()),
            &current_frame,
        );

        window_rgba
            .set_image("Terraria Preview - rgba", view_rgba)
            .unwrap_or_else(|_| {
                settings.save_to_file("settings.json");
                std::process::exit(0)
            });
    }
}
