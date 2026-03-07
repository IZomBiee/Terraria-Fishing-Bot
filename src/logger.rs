use log::{Level, Metadata, Record};
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::time::Instant;

pub struct Logger {
    pub tx: Mutex<Sender<String>>,
    pub start_time: Instant,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
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
