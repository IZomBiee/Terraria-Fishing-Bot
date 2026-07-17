use std::collections::VecDeque;

use image::{Rgb, RgbImage};
use ort::value::Value;
use ort::{session::Session, value::TensorValueType};
use rustfft::{FftPlanner, num_complex::Complex};

pub struct ONNXAudio {
    pub sample_rate: u32,
    onnx_session: Session,
}

impl ONNXAudio {
    pub fn new(onnx_path: &str, sample_rate: u32) -> Result<Self, String> {
        let onnx_session = Session::builder()
            .map_err(|e| format!("Failed to build ONNX builder: {}", e))?
            .commit_from_file(onnx_path)
            .map_err(|e| format!("Failed to load sound_detector.onnx model: {}", e))?;
        return Ok(Self {
            onnx_session,
            sample_rate,
        });
    }

    fn prepare_session_inputs(&self, snapshot: &[f32]) -> Option<Value<TensorValueType<f32>>> {
        let mel_spectrogram =
            ONNXAudio::compute_mel_spectrogram(&snapshot, self.sample_rate as f32);

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

    pub fn inference(&mut self, snapshot: &[f32]) -> Option<f32> {
        let input_tensor = self.prepare_session_inputs(snapshot)?;

        let output_tensor = self.get_output_tensor(input_tensor)?;

        let data_slice = output_tensor.try_extract_tensor::<f32>().ok()?.1;
        data_slice.get(0).copied()
    }

    pub fn render_waveform_to_image(
        snapshot: VecDeque<f32>,
        width: u32,
        height: u32,
    ) -> Option<RgbImage> {
        let mut img = RgbImage::from_pixel(width, height, Rgb([30, 30, 35]));

        let mid_y = (height / 2) as i32;
        let samples_count = snapshot.len();

        if samples_count == 0 {
            return Some(img);
        }

        for x in 0..width {
            img.put_pixel(x, mid_y as u32, Rgb([60, 60, 65]));
        }

        for x in 0..width {
            let sample_idx = ((x as f64 / width as f64) * (samples_count - 1) as f64) as usize;
            let amplitude = snapshot[sample_idx];

            let scaled_amplitude = (amplitude * (height as f32 / 2.0))
                .clamp(-(height as f32 / 2.0), height as f32 / 2.0);

            let target_y = mid_y - (scaled_amplitude as i32);

            let start_y = std::cmp::min(mid_y, target_y);
            let end_y = std::cmp::max(mid_y, target_y);

            for y in start_y..=end_y {
                if y >= 0 && y < height as i32 {
                    img.put_pixel(x, y as u32, Rgb([50, 205, 50]));
                }
            }
        }

        Some(img)
    }

    fn compute_mel_spectrogram(samples: &[f32], sample_rate: f32) -> ndarray::Array4<f32> {
        let n_fft = 1024;
        let hop_length = 512;
        let n_mels = 64;
        let n_fft_bins = n_fft / 2 + 1;

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n_fft);

        let mut frames = Vec::new();
        let mut start = 0;

        while start + n_fft <= samples.len() {
            let mut windowed = vec![Complex::new(0.0, 0.0); n_fft];
            for i in 0..n_fft {
                let hann = 0.5
                    * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n_fft - 1) as f32).cos());
                windowed[i] = Complex::new(samples[start + i] * hann, 0.0);
            }

            fft.process(&mut windowed);

            let mut power_spectrum = vec![0.0f32; n_fft_bins];
            for i in 0..n_fft_bins {
                power_spectrum[i] = windowed[i].norm_sqr();
            }
            frames.push(power_spectrum);
            start += hop_length;
        }

        let time_frames = 44;

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

        let mut mel_spectrogram = ndarray::Array4::zeros((1, 1, n_mels, time_frames));
        for t in 0..std::cmp::min(frames.len(), time_frames) {
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
