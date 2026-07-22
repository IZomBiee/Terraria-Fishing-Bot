use log::{Level, Metadata, Record};
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::time::Instant;

pub struct Logger {
    pub tx: Mutex<Sender<String>>,
    pub start_time: Instant,
}

impl Logger {
    fn is_ignored_target(target: &str) -> bool {
        target.starts_with("wgpu")
            || target.starts_with("naga")
            || target.starts_with("eframe")
            || target.starts_with("egui")
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if Self::is_ignored_target(metadata.target()) {
            return false;
        }

        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        // Check both record target and enabled state
        if Self::is_ignored_target(record.target()) {
            return;
        }

        if self.enabled(record.metadata()) {
            let msg = format!(
                "[{} - {:.2}] {}",
                record.level(),
                self.start_time.elapsed().as_secs_f32(),
                record.args()
            );

            if let Ok(sender) = self.tx.lock() {
                let _ = sender.send(msg);
            }
        }
    }

    fn flush(&self) {}
}