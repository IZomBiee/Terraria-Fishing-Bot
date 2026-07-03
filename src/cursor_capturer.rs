use crate::settings::Settings;
use image::RgbaImage;
use mouse_position::mouse_position::Mouse;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use xcap::Monitor;

pub type SharedFrame = Arc<Mutex<Option<RgbaImage>>>;

pub struct CursorCapturer {
    settings: Arc<RwLock<Settings>>,
    shared_frame: SharedFrame,
    width: u32,
    height: u32,
    monitor: Monitor,
    last_frame_start: Instant,
}

impl CursorCapturer {
    pub fn new(settings: Arc<RwLock<Settings>>, shared_frame: SharedFrame) -> Self {

        let monitor = Monitor::all()
            .expect("Could not get monitors!")
            .iter().find(|monitor| monitor.is_primary().unwrap_or(false))
            .expect("Could not find any primary screens!").clone();

        Self {
            settings,
            shared_frame,
            width: monitor.width().unwrap(),
            height: monitor.height().unwrap(),
            monitor,
            last_frame_start: Instant::now(),
        }
    }

    pub fn run(&mut self) {
        loop {
            self.last_frame_start = Instant::now();

            if let Some(frame) = self.capture_logic()
                && let Ok(mut lock) = self.shared_frame.lock()
            {
                *lock = Some(frame);
            }

            self.manage_sleep();
        }
    }

    fn capture_logic(&mut self) -> Option<RgbaImage> {
        let (x0, y0, x1, y1) = self.get_region()?;

        let frame = self.monitor.capture_region(x0, y0, x1 - x0, y1 - y0).ok()?;

        Some(frame)
    }

    fn manage_sleep(&mut self) {
        let target_fps = self.settings.read().map(|s| s.capture.fps).unwrap_or(30);
        
        if target_fps == 0 {
            std::thread::sleep(Duration::from_millis(33));
            return;
        }

        let target_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
        
        let elapsed = self.last_frame_start.elapsed();

        if elapsed < target_duration {
            let sleep_needed = target_duration - elapsed;
            std::thread::sleep(sleep_needed);
        }
    }

    pub fn get_frame(&mut self) -> Option<RgbaImage> {
        let region = self.get_region()?;
        let (x0, y0, x1, y1) = region;

        let frame = self
            .monitor
            .capture_region(x0, y0, x1 - x0, y1 - y0)
            .unwrap();

        Some(frame)
    }

    pub fn get_region(&self) -> Option<(u32, u32, u32, u32)> {
        let Mouse::Position { x, y } = Mouse::get_mouse_position() else {
            return None;
        };

        let margin = {
            let Ok(settings) = self.settings.read() else {
                return None;
            };
            settings.capture.margin as i32
        };

        let (mut x0, mut y0, mut x1, mut y1) =
            ((x - margin), (y - margin), (x + margin), (y + margin));

        if x0 < 0 { x0 = 0 };
        if y0 < 0 { y0 = 0 };
        if x1 > self.width as i32 { x1 = self.width as i32 };
        if y1 > self.height as i32 { y1 = self.height as i32 };

        if x1 - x0 <= 0 || y1 - y0 <= 0 {
            return None;
        };

        Some((x0 as u32, y0 as u32, x1 as u32, y1 as u32))
    }
}