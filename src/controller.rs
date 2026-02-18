use crate::BotSettings;
use mouse_rs::{Mouse, types::keys::Keys};
use std::thread;
use std::time::Duration;

pub struct Controller {
    pub mouse: Mouse,
    pub settings: BotSettings,
}

impl Controller {
    pub fn new(settings: BotSettings) -> Controller {
        return Controller {
            mouse: Mouse::new(),
            settings,
        };
    }

    pub fn catch(&mut self) {
        self.mouse
            .press(&Keys::LEFT)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(100));
        self.mouse
            .release(&Keys::LEFT)
            .expect("Unable to release button");
    }

    pub fn cast(&mut self) {
        self.mouse
            .press(&Keys::LEFT)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(50));
        self.mouse
            .release(&Keys::LEFT)
            .expect("Unable to release button");
    }
}
