use crate::bot::{Bot, BotState, DetectionMethod};
use crate::detectors;
use log::info;
use std::time::{Duration, Instant};

impl Bot {
    pub(crate) fn handle_liquid_calibration(&mut self, time: Instant) {
        let Some(difference_mask) = &self.get_and_show_difference_mask() else {
            return;
        };
        self.movemap_detector
            .liquid_level_calibrator
            .add(difference_mask);

        if time.elapsed()
            > Duration::from_millis(
                self.settings
                    .read()
                    .unwrap()
                    .movemap
                    .liquid_detection_delay_ms,
            )
        {
            if !self.movemap_detector.set_liquid_level() {
                info!("Liquid Calibration Result: Retry");
                self.set_state(BotState::LiquidLevelCalibration(Instant::now()));
                return;
            }
            info!(
                "Liquid Calibration Result: {}",
                self.movemap_detector.liquid_level.unwrap()
            );
            let detection_gap = self.movemap_detector.get_liquid_gap().unwrap();
            let _ = self
                .tx
                .send(crate::ui::UiSended::LiquidGap(Some(detection_gap)));
            self.set_state(BotState::Cast);
        }
    }

    pub(crate) fn handle_threshold_calibration(&mut self, time: Instant) {
        let Some(difference_mask) = self.get_and_show_difference_mask() else {
            return;
        };
        self.movemap_detector.threshold_calibrator.add(
            &difference_mask,
            self.movemap_detector.get_liquid_gap().unwrap(),
        );

        if time.elapsed()
            > Duration::from_millis(self.settings.read().unwrap().movemap.calibration_time_ms)
        {
            if !self.movemap_detector.set_threshold() {
                info!("Threshold Calibration Result: Retry");
                self.set_state(BotState::MovemapThresholdCalibration(Instant::now()));
                return;
            }
            info!(
                "Threshold Calibration Result: {}",
                self.movemap_detector.threshold.unwrap()
            );
            self.set_state(BotState::WaitingForBite(Instant::now()));
        }
    }

    pub(crate) fn handle_waiting_for_bite(&mut self, time: Instant) {
        if time.elapsed() > Duration::from_secs(30) {
            info!("Too long ({}s), recast.", time.elapsed().as_secs());
            self.set_state(BotState::Catch);
            return;
        }

        let detection_method = self.settings.read().unwrap().bot.detection_method.clone();
        match detection_method {
            DetectionMethod::Movemap => {
                if let Some(difference_mask) = self.get_and_show_difference_mask() {
                    if self.movemap_detector.is_detected(&difference_mask) {
                        self.set_state(BotState::Catch);
                    }
                }
            }
            DetectionMethod::Ocr => {
                let rgb_frame = {
                    let guard = self.shared_frame.lock().unwrap();
                    let Some(ref frame) = *guard else {
                        return;
                    };
                    detectors::ocr::rgba_2_rgb(frame)
                };

                let detected_word = self.ocr_detector.get_strings_from_frame(rgb_frame);
                let (correct_detection_words, threshold) = {
                    let settings = self.settings.read().unwrap();
                    (
                        settings.ocr.sonar_detection_words.clone(),
                        settings.ocr.sonar_detection_threshold,
                    )
                };

                if detectors::ocr::Ocr::is_needed_string(
                    &correct_detection_words,
                    &detected_word,
                    threshold,
                ) {
                    info!("Detected Word {}", detected_word);
                    self.set_state(BotState::Catch);
                }
            }
            DetectionMethod::Audio => {
                let confidence = self.audio_detector.inference().unwrap_or(0.0);
                if confidence > (self.settings.read().unwrap().audio.min_confidence as f32 / 100.0)
                {
                    info!(
                        "Detected sound with {}% confidence.",
                        (confidence * 100.0).round()
                    );
                    self.set_state(BotState::Catch);
                }
            }
        }
    }
}
