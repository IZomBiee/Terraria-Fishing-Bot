use crate::settings::Settings;
use mouse_rs::{Mouse, types::keys::Keys};
use std::thread;
use std::time::Duration;

pub struct Controller<'a> {
    pub mouse: Mouse,
    pub settings: &'a Settings,
}

impl<'a> Controller<'a> {
    pub fn new(settings: &'_ Settings) -> Controller<'_> {
        Controller {
            mouse: Mouse::new(),
            settings,
        }
    }

    pub fn catch(&self) {
        self.mouse
            .press(&Keys::LEFT)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(100));
        self.mouse
            .release(&Keys::LEFT)
            .expect("Unable to release button");
    }

    pub fn cast(&self) {
        self.mouse
            .press(&Keys::LEFT)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(50));
        self.mouse
            .release(&Keys::LEFT)
            .expect("Unable to release button");
    }
}
