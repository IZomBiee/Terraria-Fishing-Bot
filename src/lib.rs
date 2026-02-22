pub mod bot;
pub mod controller;
pub mod cursor_capturer;
pub mod opencv;

#[derive(Clone, Copy)]
pub struct BotSettings {
    pub margin: u32,
    pub monitor_id: u8,
    pub fps: u8,
    pub casting_delay_millis: u64,
    pub catch_threshold: f32,
    pub liquid_threshold: f32,
    pub liquid_offset: f32,
    pub detection_gap_size: f32,
}

impl BotSettings {
    pub fn default() -> BotSettings {
        BotSettings {
            margin: 100,
            monitor_id: 1,
            fps: 5,
            casting_delay_millis: 1000,
            catch_threshold: 0.3,
            liquid_threshold: 0.5,
            liquid_offset: -0.02,
            detection_gap_size: 0.05,
        }
    }
}
