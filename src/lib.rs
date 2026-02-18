pub mod bot;
pub mod controller;
pub mod cursor_capturer;
pub mod detector;
pub mod opencv;

#[derive(Clone, Copy)]
pub struct BotSettings {
    pub margin: u32,
    pub monitor_id: u8,
    pub fps: u8,
    pub casting_delay: u64,
    pub reeling_delay: u64,
    pub hsv_min: [u8; 3],
    pub hsv_max: [u8; 3],
    pub catch_thresh: u32,
}
