use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use crate::settings::Settings;

struct AudioServer {
    settings: Arc<RwLock<Settings>>,
    max_samples: usize,
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
    stream: cpal::Stream,
}

impl AudioServer {
    pub fn new(settings: Arc<RwLock<Settings>>) -> Result<Self, String> {
        let max_samples = {
            let settings = &settings.read().unwrap().audio;
            ((settings.sample_rate as u128 * settings.buffer_duration.as_millis()) / 1000) as usize
        };

        let ring_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(max_samples)));
        let ring_buffer_clone = Arc::clone(&ring_buffer);

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No ouput audio device found!".to_string())?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default output config: {}", e))?;

        let native_format: cpal::StreamConfig = supported_config.into();
        let native_sample_rate = native_format.sample_rate as f32;
        let native_channels = native_format.channels as usize;

        let target_sample_rate = settings.read().unwrap().audio.sample_rate;

        let stream = device
            .build_input_stream(
                native_format,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buffer = ring_buffer_clone.lock().unwrap();

                    for frame in data.chunks(native_channels) {
                        let mono_sample: f32 = frame.iter().sum::<f32>() / native_channels as f32;

                        static mut ACCUMULATOR: f32 = 0.0;
                        unsafe {
                            ACCUMULATOR += target_sample_rate as f32;
                            if ACCUMULATOR >= native_sample_rate {
                                ACCUMULATOR -= native_sample_rate;

                                buffer.push_back(mono_sample);
                                if buffer.len() > max_samples {
                                    buffer.pop_front();
                                }
                            }
                        }
                    }
                },
                |err| log::error!("An error occurred on the audio stream: {}", err),
                None,
            )
            .map_err(|e| format!("Failed to build audio stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start audio stream: {}", e))?;

        Ok(Self {
            settings,
            max_samples,
            ring_buffer,
            stream,
        })
    }

    pub fn get_buffer(&self) -> Option<VecDeque<f32>> {
        let lock = self.ring_buffer.try_lock().ok()?;
        return Some(lock.clone());
    }
}
