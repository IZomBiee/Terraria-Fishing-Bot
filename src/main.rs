use show_image::{ImageView, create_window};
use terraria_fishing_bot::BotSettings;
use terraria_fishing_bot::bot::Bot;
use terraria_fishing_bot::controller::Controller;
use terraria_fishing_bot::cursor_capturer::CursorCapturer;
use terraria_fishing_bot::opencv::{hsva_in_range, rgba_2_hsva};

#[show_image::main]
fn main() {
    let window = create_window("Terraria Preview", Default::default()).unwrap();
    let window_mask = create_window("Terraria Preview - Mask", Default::default()).unwrap();
    let settings = BotSettings {
        margin: 70,
        monitor_id: 1,
        fps: 20,
        casting_delay: 1500,
        reeling_delay: 1500,
        hsv_min: [0, 88, 140],
        hsv_max: [17, 255, 255],
        catch_thresh: 15,
    };
    let mut capturer = CursorCapturer::new(settings);
    let controller = Controller::new(settings);

    let mut bot = Bot::new(settings, controller);

    bot.start();

    loop {
        let Some((mut rgba_frame, (width, height))) = capturer.get_frame() else {
            continue;
        };

        let hsv_frame = rgba_2_hsva(&rgba_frame);
        let mask = hsva_in_range(&hsv_frame, settings.hsv_min, settings.hsv_max);

        bot.update(&mask, width, height);
        bot.draw_last_pos(&mut rgba_frame, width, height);

        let view = ImageView::new(show_image::ImageInfo::rgba8(width, height), &rgba_frame);

        window.set_image("Terraria Preview", view).unwrap();

        let view_mask = ImageView::new(show_image::ImageInfo::mono8(width, height), &mask);

        window_mask
            .set_image("Terraria Preview - Mask", view_mask)
            .unwrap();
    }
}
