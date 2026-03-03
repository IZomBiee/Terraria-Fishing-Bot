use crate::settings::Settings;
use image::RgbaImage;
use mouse_position::mouse_position::Mouse;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use xcap::Monitor;

pub type SharedFrame = Arc<Mutex<Option<RgbaImage>>>;

pub struct CursorCapturer {
    settings: Arc<Mutex<Settings>>,
    shared_frame: SharedFrame,
    width: u32,
    height: u32,
    monitor: Monitor,
    sleep_time: Duration,
    frame_history: VecDeque<Instant>,
}

impl CursorCapturer {
    pub fn new(settings: Arc<Mutex<Settings>>, shared_frame: SharedFrame) -> Self {
        let monitor = Monitor::all()
            .unwrap()
            .first()
            .expect("The monitor was not found!")
            .clone();

        Self {
            settings,
            shared_frame,
            width: monitor.width().unwrap(),
            height: monitor.height().unwrap(),
            monitor,
            sleep_time: Duration::from_millis(1),
            frame_history: VecDeque::with_capacity(10),
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Some(frame) = self.capture_logic()
                && let Ok(mut lock) = self.shared_frame.lock()
            {
                *lock = Some(frame);
            }

            self.manage_sleep();
            std::thread::sleep(self.sleep_time);
        }
    }

    fn capture_logic(&mut self) -> Option<RgbaImage> {
        let (x0, y0, x1, y1) = self.get_region()?;

        let frame = self.monitor.capture_region(x0, y0, x1 - x0, y1 - y0).ok()?; // Use ok() to handle errors gracefully

        self.add_frame_history();
        Some(frame)
    }

    fn manage_sleep(&mut self) {
        let target_fps = self.settings.lock().map(|s| s.fps).unwrap_or(30);
        let current_fps = self.get_fps();

        if (current_fps as u8) > target_fps {
            self.sleep_time += Duration::from_micros(500);
        } else if self.sleep_time.as_micros() >= 500 {
            self.sleep_time -= Duration::from_micros(500);
        }
    }

    fn add_frame_history(&mut self) {
        if self.frame_history.len() >= 10 {
            self.frame_history.pop_front();
        }

        self.frame_history.push_back(Instant::now());
    }

    pub fn get_fps(&self) -> f32 {
        if self.frame_history.len() >= 2 {
            let first = self.frame_history.front().unwrap();

            let last = self.frame_history.back().unwrap();

            let duration = *last - *first;

            let count = self.frame_history.len() as f32;

            return count / duration.as_secs_f32();
        }

        0f32
    }

    pub fn get_frame(&mut self) -> Option<RgbaImage> {
        let region = self.get_region()?;

        let (x0, y0, x1, y1) = region;

        let frame = self
            .monitor
            .capture_region(x0, y0, x1 - x0, y1 - y0)
            .unwrap();

        let target_fps = {
            let Ok(settings) = self.settings.lock() else {
                return None;
            };

            settings.fps
        };

        if (self.get_fps() as u8) > target_fps {
            self.sleep_time += Duration::from_millis(1);
        } else if self.sleep_time.as_millis() >= 5 {
            self.sleep_time -= Duration::from_millis(1);
        }

        std::thread::sleep(self.sleep_time);

        self.add_frame_history();

        Some(frame)
    }

    pub fn get_region(&self) -> Option<(u32, u32, u32, u32)> {
        let Mouse::Position { x, y } = Mouse::get_mouse_position() else {
            return None;
        };

        let margin = {
            let Ok(settings) = self.settings.lock() else {
                return None;
            };
            settings.margin as i32
        };

        let (mut x0, mut y0, mut x1, mut y1) =
            ((x - margin), (y - margin), (x + margin), (y + margin));

        if x0 < 0 {
            x0 = 0
        };
        if y0 < 0 {
            y0 = 0
        };
        if x1 > self.width as i32 {
            x1 = self.width as i32
        };
        if y1 > self.height as i32 {
            y1 = self.height as i32
        };

        if x1 - x0 <= 0 || y1 - y0 <= 0 {
            return None;
        };

        Some((x0 as u32, y0 as u32, x1 as u32, y1 as u32))
    }
}
