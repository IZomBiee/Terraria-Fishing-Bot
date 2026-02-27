use crate::settings::Settings;
use enigo::{Enigo, Key, Keyboard};
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

    pub fn use_potions(&self) {
        return;
        let mut enigo = Enigo::new(&enigo::Settings::default()).unwrap();

        if enigo
            .key(Key::Unicode('b'), enigo::Direction::Press)
            .is_ok()
        {
            thread::sleep(Duration::from_millis(100));
            enigo
                .key(Key::Unicode('b'), enigo::Direction::Release)
                .unwrap_or_else(|_| {
                    println!("Can't release potion button!");
                })
        } else {
            println!("Can't use potions!");
        }
        println!("Used potion!");
    }
}
