use device_query::{DeviceQuery, DeviceState, Keycode};
use show_image::{ImageView, create_window};
use terraria_fishing_bot::BotSettings;
use terraria_fishing_bot::bot::Bot;
use terraria_fishing_bot::controller::Controller;
use terraria_fishing_bot::cursor_capturer::CursorCapturer;
use terraria_fishing_bot::opencv;
use image::{RgbaImage};

#[show_image::main]
fn main() {
    let window = create_window("Terraria Preview", Default::default()).unwrap();

    let settings = BotSettings::default();
    let mut capturer = CursorCapturer::new(settings);
    let controller = Controller::new(settings);

    let mut bot = Bot::new(settings, Some(controller));

    let mut last_frame: Option<RgbaImage> = None;

    let device_state = DeviceState::new();

    println!("Press Q to start, R to stop.");

    loop {
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::Q) {
            bot.start();
        } else if keys.contains(&Keycode::R) {
            bot.stop();
        }

        let Some(current_frame ) = capturer.get_frame()
        else {
            continue;
        };

        if let Some(last_frame) = &last_frame
            && current_frame.dimensions() == last_frame.dimensions() {
                let mask = opencv::rgba_difference_mask(&current_frame, last_frame);

                let mut draw_frame = current_frame.clone();

                bot.update(&mask);

                bot.draw_detection_gap(&mut draw_frame);

                let view = ImageView::new(
                    show_image::ImageInfo::rgba8(draw_frame.width(), draw_frame.height()),
                    &draw_frame,
                );

                window.set_image("Terraria Preview", view).unwrap();
            };

        last_frame = Some(current_frame);
    }
}
