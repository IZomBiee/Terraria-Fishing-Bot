use device_query::{DeviceQuery, DeviceState, Keycode};
use image::RgbaImage;
use show_image::{ImageView, create_window};
use terraria_fishing_bot::bot::Bot;
use terraria_fishing_bot::controller::Controller;
use terraria_fishing_bot::cursor_capturer::CursorCapturer;
use terraria_fishing_bot::opencv;
use terraria_fishing_bot::settings::Settings;

#[show_image::main]
fn main() {
    let window_settings = show_image::WindowOptions {
        size: Some([500, 500]),
        ..Default::default()
    };

    let window_mask = create_window("Terraria Preview - mask", window_settings.clone()).unwrap();
    let window_rgba = create_window("Terraria Preview - rgba", window_settings).unwrap();

    let settings = Settings::default();
    let mut capturer = CursorCapturer::new(&settings);
    let controller = Controller::new(&settings);

    let mut bot = Bot::new(&settings, Some(controller));

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

        let Some(current_frame) = capturer.get_frame() else {
            continue;
        };

        if let Some(last_frame) = &last_frame
            && current_frame.dimensions() == last_frame.dimensions()
        {
            let mask = opencv::rgba_difference_mask(&current_frame, last_frame);

            let mut draw_frame = current_frame.clone();

            bot.update(&mask);

            bot.draw_detection_gap(&mut draw_frame);

            let view_mask = ImageView::new(
                show_image::ImageInfo::mono8(draw_frame.width(), draw_frame.height()),
                &mask,
            );

            let view_rgba = ImageView::new(
                show_image::ImageInfo::rgba8(draw_frame.width(), draw_frame.height()),
                &draw_frame,
            );

            window_mask
                .set_image("Terraria Preview - mask", view_mask)
                .unwrap();
            window_rgba
                .set_image("Terraria Preview - rgba", view_rgba)
                .unwrap();
        };

        last_frame = Some(current_frame);
    }
}
