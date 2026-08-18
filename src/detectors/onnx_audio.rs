use image::{Rgba, RgbaImage};
use ndarray::Array4;
use ort::value::Value;
use ort::{session::Session, value::TensorValueType};
use rustfft::{FftPlanner, num_complex::Complex};

use crate::audio_server::AudioServer;

static MODEL_BYTES: &[u8] = include_bytes!("../../assets/sound_detector.onnx");

pub struct OnnxAudio {
    pub sample_rate: u32,
    pub last_confidence_level: f32,
    audio_server: AudioServer,
    onnx_session: Session,
}

impl OnnxAudio {
    pub fn new(
        audio_server: AudioServer,
        sample_rate: u32,
    ) -> Result<Self, String> {
        let onnx_session = Session::builder()
            .map_err(|e| format!("Failed to build ONNX builder: {}", e))?
            .commit_from_memory(MODEL_BYTES)
            .map_err(|e| format!("Failed to load sound_detector.onnx model: {}", e))?;
        return Ok(Self {
            onnx_session,
            last_confidence_level: 0.0,
            audio_server,
            sample_rate,
        });
    }

    fn prepare_session_inputs(&self, snapshot: &[f32]) -> Option<Value<TensorValueType<f32>>> {
        let mel_spectrogram =
            OnnxAudio::compute_mel_spectrogram(&snapshot, self.sample_rate as f32);

        Value::from_array(mel_spectrogram).ok()
    }

    fn get_output_tensor(
        &mut self,
        input_tensor: Value<TensorValueType<f32>>,
    ) -> Option<ort::value::Value> {
        let session_inputs = ort::inputs![input_tensor];

        let mut session_output = self.onnx_session.run(session_inputs).ok()?;

        session_output.remove("output")
    }

    pub fn update(&mut self) {
        let Some(mut snapshot) = self.audio_server.get_buffer() else {
            return;
        };

        let input_tensor = self.prepare_session_inputs(&snapshot.make_contiguous()).unwrap();

        let Some(output_tensor) = self.get_output_tensor(input_tensor) else {
            self.last_confidence_level = 0.0;
            return;
        };

        let data_slice = output_tensor.try_extract_tensor::<f32>().unwrap().1;

        self.last_confidence_level = data_slice.get(0).copied().unwrap_or(0.0);
    }

    pub fn render_waveform(&self, width: u32, height: u32) -> Option<RgbaImage> {
        let mut img = RgbaImage::from_pixel(width, height, Rgba([30, 30, 35, 255]));

        let Some(mut snapshot) = self.audio_server.get_buffer() else {
            return None;
        };

        let snapshot = snapshot.make_contiguous();

        let mid_y = (height / 2) as i32;
        let samples_count = snapshot.len();

        if samples_count == 0 {
            return Some(img);
        }

        for x in 0..width {
            img.put_pixel(x, mid_y as u32, Rgba([60, 60, 65, 255]));
        }

        for x in 0..width {
            let sample_idx = ((x as f64 / width as f64) * (samples_count - 1) as f64) as usize;
            let amplitude = snapshot[sample_idx];

            let scaled_amplitude = (amplitude * (height as f32 / 2.0))
                .clamp(-(height as f32 / 2.0), height as f32 / 2.0);

            let target_y = mid_y - (scaled_amplitude as i32) * 4;

            let start_y = std::cmp::min(mid_y, target_y);
            let end_y = std::cmp::max(mid_y, target_y);

            for y in start_y..=end_y {
                if y >= 0 && y < height as i32 {
                    img.put_pixel(x, y as u32, Rgba([50, 205, 50, 255]));
                }
            }
        }

        Some(img)
    }

    fn compute_mel_spectrogram(samples: &[f32], sample_rate: f32) -> Array4<f32> {
        let n_fft = 1024 * 4; // 4096
        let hop_length = 512 * 4; // 2048
        let n_mels = 64 * 4; // 256
        let n_fft_bins = n_fft / 2 + 1;

        // 1. CENTER PADDING (Matches PyTorch's default center=True behavior)
        let pad_len = n_fft / 2;
        let mut padded_samples = Vec::with_capacity(samples.len() + 2 * pad_len);
        padded_samples.resize(pad_len, 0.0);
        padded_samples.extend_from_slice(samples);
        padded_samples.resize(samples.len() + 2 * pad_len, 0.0);

        // 2. DYNAMICALLY CALCULATE EXPECTED STFT FRAMES
        let expected_time_frames = if padded_samples.len() >= n_fft {
            1 + (padded_samples.len() - n_fft) / hop_length
        } else {
            0
        };

        // 3. STFT & POWER SPECTRUM
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n_fft);

        let mut frames = Vec::with_capacity(expected_time_frames);
        let mut start = 0;

        while start + n_fft <= padded_samples.len() {
            let mut windowed = vec![Complex::new(0.0, 0.0); n_fft];
            for i in 0..n_fft {
                // Hann Window
                let hann = 0.5
                    * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n_fft - 1) as f32).cos());
                windowed[i] = Complex::new(padded_samples[start + i] * hann, 0.0);
            }

            fft.process(&mut windowed);

            let mut power_spectrum = vec![0.0f32; n_fft_bins];
            for i in 0..n_fft_bins {
                power_spectrum[i] = windowed[i].norm_sqr();
            }
            frames.push(power_spectrum);
            start += hop_length;
        }

        // 4. MEL FILTERBANK CONSTRUCTION
        let hz_to_mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
        let mel_to_hz = |mel: f32| 700.0 * (10.0f32.powf(mel / 2595.0) - 1.0);

        let min_mel = hz_to_mel(0.0);
        let max_mel = hz_to_mel(sample_rate / 2.0);

        let mut mel_points = vec![0.0f32; n_mels + 2];
        for i in 0..(n_mels + 2) {
            let fraction = i as f32 / (n_mels + 1) as f32;
            mel_points[i] = mel_to_hz(min_mel + fraction * (max_mel - min_mel));
        }

        let mut fb = vec![vec![0.0f32; n_fft_bins]; n_mels];
        for m in 0..n_mels {
            let f_minus = mel_points[m];
            let f_center = mel_points[m + 1];
            let f_plus = mel_points[m + 2];

            for k in 0..n_fft_bins {
                let hz = (k as f32 * sample_rate) / n_fft as f32;
                if hz > f_minus && hz < f_center {
                    fb[m][k] = (hz - f_minus) / (f_center - f_minus);
                } else if hz >= f_center && hz < f_plus {
                    fb[m][k] = (f_plus - hz) / (f_plus - f_center);
                }
            }
        }

        // 5. BUILD TENSOR [1, 1, n_mels, time_frames]
        let actual_time_frames = frames.len();
        let mut mel_spectrogram = Array4::zeros((1, 1, n_mels, actual_time_frames));

        for t in 0..actual_time_frames {
            for m in 0..n_mels {
                let mut mel_energy = 0.0f32;
                for k in 0..n_fft_bins {
                    mel_energy += frames[t][k] * fb[m][k];
                }
                let db = 10.0 * (mel_energy.max(1e-10)).log10();
                mel_spectrogram[[0, 0, m, t]] = db;
            }
        }

        mel_spectrogram
    }
}
