use crate::bot::{Bot, BotState, DetectionMethod};
use log::info;
use std::time::{Duration, Instant};

impl Bot {
    pub(crate) fn handle_liquid_calibration(&mut self, time: Instant) {
        let Some(difference_mask) = &self.movemap_detector.last_difference_mask else {
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

            self.set_state(BotState::Cast);
        }
    }

    pub(crate) fn handle_threshold_calibration(&mut self, time: Instant) {
        let Some(difference_mask) = &self.movemap_detector.last_difference_mask else {
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
                if self.movemap_detector.is_detected() {
                    self.set_state(BotState::Catch);
                }
            }
            DetectionMethod::Ocr => {
                let (correct_detection_words, threshold) = {
                    let settings = self.settings.read().unwrap();
                    (
                        settings.ocr.sonar_detection_words.clone(),
                        settings.ocr.sonar_detection_threshold,
                    )
                };

                if self.ocr_detector.is_needed_string(
                    &correct_detection_words,
                    threshold,
                ) {
                    info!("Detected Word {}", self.ocr_detector.last_detected_text.to_owned().unwrap_or("".to_owned()));
                    self.set_state(BotState::Catch);
                }
            }
            DetectionMethod::Audio => {
                let confidence = self.audio_detector.last_confidence_level;
                if confidence > (self.settings.read().unwrap().audio.min_confidence as f32 / 100.0)
                {
                    info!(
                        "Detected sound with {}% confidence.",
                        (confidence * 100.0).round()
                    );
                    self.set_state(BotState::Catch);
                }
            }
            DetectionMethod::YOLO => {
                let Some(ref detection) = self.yolo_detector.last_detection else {
                    return;
                };

                let Some(ref previous_detection) = self.yolo_detector.previous_detection else {
                    return;
                };

                let dx = previous_detection.y0 as i32 - detection.y0 as i32;
                let dy = previous_detection.x0 as i32 - detection.x0 as i32;
                let diff = dx.abs() + dy.abs();

                if diff > self.settings.read().unwrap().yolo.threshold as i32 {
                    self.set_state(BotState::Catch);
                }
            }
        }
    }
}
