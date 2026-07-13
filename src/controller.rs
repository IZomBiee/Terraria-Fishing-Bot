use crate::settings::Settings;
use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings};
use log::{error, info};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

pub struct Controller {
    pub enigo: Enigo,
    pub settings: Arc<RwLock<Settings>>,
    pub last_potion_use_time: Option<Instant>,
}

impl Controller {
    pub fn new(settings: Arc<RwLock<Settings>>) -> Controller {
        let enigo = Enigo::new(&EnigoSettings::default()).expect("Could not initialize Enigo");
        Controller {
            enigo,
            settings,
            last_potion_use_time: None,
        }
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
            self.last_potion_use_time = Some(Instant::now());
            thread::sleep(Duration::from_millis(100));
            self.enigo
                .key(Key::Unicode('b'), Direction::Release)
                .unwrap_or_else(|_| {
                    error!("Can't release potion button!");
                })
        } else {
            error!("Can't use potions!");
        }
        info!("Used potion!");
    }

    pub fn use_potions_if_necessary(&mut self) {
        if self.last_potion_use_time.is_none()
            || self.last_potion_use_time.unwrap().elapsed()
                > self.settings.read().unwrap().bot.potion_use_delay
        {
            self.use_potions();
        }
    }
}
