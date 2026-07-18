use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings};
use log::error;
use std::thread;
use std::time::Duration;

pub struct Controller {
    pub enigo: Enigo,
}

impl Controller {
    pub fn new() -> Controller {
        let enigo = Enigo::new(&EnigoSettings::default()).expect("Could not initialize Enigo");
        Controller { enigo }
    }

    pub fn catch(&mut self) {
        self.enigo
            .button(Button::Left, Direction::Press)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(50));
        self.enigo
            .button(Button::Left, Direction::Release)
            .expect("Unable to release button");
    }

    pub fn cast(&mut self) {
        self.enigo
            .button(Button::Left, Direction::Press)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(50));
        self.enigo
            .button(Button::Left, Direction::Release)
            .expect("Unable to release button");
    }

    pub fn use_potions(&mut self) {
        if self.enigo.key(Key::Unicode('b'), Direction::Press).is_ok() {
            thread::sleep(Duration::from_millis(100));
            self.enigo
                .key(Key::Unicode('b'), Direction::Release)
                .unwrap_or_else(|_| {
                    error!("Can't release potion button!");
                })
        } else {
            error!("Can't use potions!");
        }
    }
}
