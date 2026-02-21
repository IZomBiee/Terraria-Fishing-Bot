use device_query::{DeviceQuery, DeviceState, Keycode};
use show_image::{ImageView, create_window};
use terraria_fishing_bot::BotSettings;
use terraria_fishing_bot::bot::Bot;
use terraria_fishing_bot::controller::Controller;
use terraria_fishing_bot::cursor_capturer::CursorCapturer;
use terraria_fishing_bot::opencv;

#[show_image::main]
fn main() {
    let window = create_window("Terraria Preview", Default::default()).unwrap();

    let settings = BotSettings {
        margin: 100,
        monitor_id: 1,
        fps: 5,
        casting_delay: 1000,
        hsv_min: [0, 88, 140],
        hsv_max: [17, 255, 255],
        catch_threshold: 200,
        liquid_threshold: 100,
        liquid_offset: -5,
        liquid_gap: 20,
    };
    let mut capturer = CursorCapturer::new(settings);
    let controller = Controller::new(settings);

    let mut bot = Bot::new(settings, controller);

    let mut last_rgba_frame: Option<(Vec<u8>, (u32, u32))> = None;

    let device_state = DeviceState::new();

    println!("Press Q to start, R to stop.");

    loop {
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::Q) {
            bot.start();
        } else if keys.contains(&Keycode::R) {
            bot.stop();
        }

        let Some((current_rgba_frame, (current_width, current_height))) = capturer.get_frame()
        else {
            continue;
        };

        if let Some((last_rgba_frame, (last_width, last_height))) = &last_rgba_frame {
            if current_width == *last_width && current_height == *last_height {
                let mask = opencv::rgba_difference_mask(&current_rgba_frame, last_rgba_frame);

                let mut draw_rgba_frame = current_rgba_frame.clone();

                bot.update(&mask, current_width, current_height);

                bot.draw_detection_gap(&mut draw_rgba_frame, current_width, current_height);

                let view = ImageView::new(
                    show_image::ImageInfo::rgba8(current_width, current_height),
                    &draw_rgba_frame,
                );

                // let view = ImageView::new(
                //     show_image::ImageInfo::mono8(current_width, current_height),
                //     &mask,
                // );

                window.set_image("Terraria Preview", view).unwrap();
            }
        };

        last_rgba_frame = Some((current_rgba_frame, (current_width, current_height)));
    }
}
