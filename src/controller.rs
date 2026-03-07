use crate::settings::Settings;
use enigo::{Enigo, Key, Keyboard};
use log::{error, info};
use mouse_rs::{Mouse, types::keys::Keys};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

pub struct Controller {
    pub mouse: Mouse,
    pub settings: Arc<RwLock<Settings>>,
    pub last_potion_use_time: Option<Instant>,
}

impl Controller {
    pub fn new(settings: Arc<RwLock<Settings>>) -> Controller {
        Controller {
            mouse: Mouse::new(),
            settings,
            last_potion_use_time: None,
        }
    }

    pub fn catch(&self) {
        self.mouse
            .press(&Keys::LEFT)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(150));
        self.mouse
            .release(&Keys::LEFT)
            .expect("Unable to release button");
        thread::sleep(Duration::from_millis(50));
    }

    pub fn cast(&self) {
        self.mouse
            .press(&Keys::LEFT)
            .expect("Unable to press button");
        thread::sleep(Duration::from_millis(150));
        self.mouse
            .release(&Keys::LEFT)
            .expect("Unable to release button");
        thread::sleep(Duration::from_millis(50));
    }

    pub fn use_potions(&mut self) {
        let mut enigo = Enigo::new(&enigo::Settings::default()).unwrap();

        if enigo
            .key(Key::Unicode('b'), enigo::Direction::Press)
            .is_ok()
        {
            self.last_potion_use_time = Some(Instant::now());
            thread::sleep(Duration::from_millis(100));
            enigo
                .key(Key::Unicode('b'), enigo::Direction::Release)
                .unwrap_or_else(|_| {
                    error!("Can't release potion button!");
                })
        } else {
            error!("Can't use potions!");
        }
        info!("Used potion!");
    }

    pub fn use_potions_if_necessery(&mut self) {
        if self.last_potion_use_time.is_none()
            || self.last_potion_use_time.unwrap().elapsed()
                > self.settings.read().unwrap().bot.potion_use_delay
        {
            self.use_potions();
        }
    }
}
