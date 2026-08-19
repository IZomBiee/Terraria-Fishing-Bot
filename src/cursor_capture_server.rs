use crate::bot::types::DetectionMethod;
use crate::fps_handler::FpsHandler;
use crate::settings::Settings;
use crate::ui::UiSended;
use enigo::{Enigo, Mouse, Settings as EnigoSettings};
use image::RgbaImage;
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use xcap::Monitor;

pub type SharedFrame = Arc<Mutex<Option<RgbaImage>>>;

#[derive(Debug, Clone, PartialEq)]
struct CursorLocation {
    monitor: usize,
    x: u32,
    y: u32,
}

pub struct CursorCaptureServer {
    tx: mpsc::Sender<UiSended>,
    settings: Arc<RwLock<Settings>>,
    output_frame: SharedFrame,
    monitors: Vec<Monitor>,
    last_frame_start: Instant,
    fps_handler: FpsHandler,
    enigo: Enigo,
}

impl CursorCaptureServer {
    pub fn new(
        tx: mpsc::Sender<UiSended>,
        settings: Arc<RwLock<Settings>>,
        output_frame: SharedFrame,
    ) -> Self {
        let mut monitors = Monitor::all().expect("Could not get monitors!");
        monitors.sort_by_key(|monitor| monitor.x().unwrap_or(0));

        let enigo = Enigo::new(&EnigoSettings::default()).expect("Could not initialize Enigo");

        Self {
            tx,
            settings,
            output_frame,
            monitors,
            last_frame_start: Instant::now(),
            fps_handler: FpsHandler::default(),
            enigo,
        }
    }

    fn get_cursor_location(&self) -> CursorLocation {
        let mut monitors_x = Vec::<u32>::new();

        if self.monitors.len() > 1 {
            for monitor in &self.monitors[1..] {
                monitors_x.push(monitor.x().unwrap() as u32);
            }
        }

        let (mut mouse_x, mouse_y) = self.enigo.location().unwrap();

        let mut monitor_id: usize = 0;
        for x in monitors_x {
            if mouse_x > x as i32 {
                monitor_id += 1;
                mouse_x -= x as i32;
            }
        }

        let x = mouse_x as u32;
        let y = (mouse_y as f32 - self.monitors[monitor_id].y().unwrap_or(0) as f32) as u32;

        return CursorLocation {
            monitor: monitor_id,
            x,
            y,
        };
    }

    pub fn run(&mut self) {
        let mut counter = 0;
        let mut last_time = Instant::now();
        loop {
            counter += 1;
            self.last_frame_start = Instant::now();

            let target_fps = self.settings.read().map(|s| s.capture.fps).unwrap_or(30);

            if target_fps == 0
                || self.settings.read().unwrap().bot.detection_method == DetectionMethod::Audio
            {
                thread::sleep(Duration::from_millis(100));
                let _ = self.tx.send(UiSended::SetCaptureFps(0.0));
                continue;
            }

            if let Some(frame) = self.capture_logic()
                && let Ok(mut lock) = self.output_frame.lock()
            {
                if last_time.elapsed() > Duration::from_secs(1) {
                    let _ = frame.save(format!(
                        "C:\\Users\\patri\\Desktop\\Bobbers\\{}.png",
                        counter
                    ));
                    last_time = Instant::now();
                }
                *lock = Some(frame);
            }

            self.manage_sleep(target_fps);

            self.fps_handler.add(self.last_frame_start);

            if let Some(fps) = self.fps_handler.get_avg_fps() {
                let _ = self.tx.send(UiSended::SetCaptureFps(fps));
            }
        }
    }

    fn capture_logic(&mut self) -> Option<RgbaImage> {
        let cursor_location = self.get_cursor_location();

        let (x0, y0, x1, y1) = self.get_region(&cursor_location)?;

        let frame = self.monitors[cursor_location.monitor]
            .capture_region(x0, y0, x1 - x0, y1 - y0)
            .ok()?;

        Some(frame)
    }

    fn manage_sleep(&mut self, target_fps: u8) {
        let target_duration = Duration::from_secs_f32(1.0 / target_fps as f32);

        let elapsed = self.last_frame_start.elapsed();

        if elapsed < target_duration {
            let sleep_needed = target_duration - elapsed;
            std::thread::sleep(sleep_needed);
        }
    }

    fn get_region(&self, cursor_location: &CursorLocation) -> Option<(u32, u32, u32, u32)> {
        let margin = {
            let Ok(settings) = self.settings.read() else {
                return None;
            };
            settings.capture.margin as i32
        };

        let (mut x0, mut y0, mut x1, mut y1) = (
            (cursor_location.x as i32 - margin),
            (cursor_location.y as i32 - margin),
            (cursor_location.x as i32 + margin),
            (cursor_location.y as i32 + margin),
        );

        if x0 < 0 {
            x1 -= x0;
            x0 = 0
        };
        if y0 < 0 {
            y1 -= y0;
            y0 = 0
        };

        let width = self.monitors[cursor_location.monitor].width().unwrap();
        let height = self.monitors[cursor_location.monitor].height().unwrap();

        if x1 > width as i32 {
            x0 += width as i32 - x1;
            x1 = width as i32
        };
        if y1 > height as i32 {
            y0 += height as i32 - y1;
            y1 = height as i32
        };

        if x1 - x0 <= 0 || y1 - y0 <= 0 {
            return None;
        };

        Some((x0 as u32, y0 as u32, x1 as u32, y1 as u32))
    }
}
