use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub struct FpsHandler {
    buffer_duration: Duration,
    frametime_buffer: VecDeque<Duration>,
}

impl Default for FpsHandler {
    fn default() -> Self {
        Self {
            frametime_buffer: VecDeque::new(),
            buffer_duration: Duration::from_secs(1),
        }
    }
}

impl FpsHandler {
    pub fn add(&mut self, time: Instant) {
        self.frametime_buffer.push_front(time.elapsed());

        if self.get_buffer_duration() > self.buffer_duration {
            let _ = self.frametime_buffer.pop_back();
        }
    }

    fn get_buffer_duration(&self) -> Duration {
        if self.frametime_buffer.is_empty() {
            return Duration::from_secs(0);
        }

        let buffer_duration = self.frametime_buffer
            .iter().sum::<Duration>();

        buffer_duration
    }

    pub fn get_avg_fps(&self) -> Option<f32> {
        if self.frametime_buffer.is_empty() {
            return None;
        }
        let fps = self.frametime_buffer
                .iter()
                .map(|duration| duration.as_millis() as f32)
                .sum::<f32>()
                / (self.frametime_buffer.len() as f32);

        return Some(
            1000.0/fps
        );
    }
}
