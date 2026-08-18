use std::sync::{Arc, RwLock};

use crate::settings::Settings;
use image::{GrayImage, Luma, RgbaImage, imageops};
use log::info;

#[derive(Debug)]
pub struct Movemap {
    pub liquid_level: Option<u32>,
    pub threshold: Option<u32>,
    settings: Arc<RwLock<Settings>>,
    last_frame: Option<RgbaImage>,
    pub last_difference_mask: Option<GrayImage>,
    pub threshold_calibrator: ThresholdCalibrator,
    pub liquid_level_calibrator: LiquidLevelCalibrator,
}

impl Movemap {
    pub fn new(settings: Arc<RwLock<Settings>>) -> Self {
        Self {
            settings,
            last_frame: None,
            liquid_level: None,
            threshold: None,
            last_difference_mask: None,
            threshold_calibrator: ThresholdCalibrator::default(),
            liquid_level_calibrator: LiquidLevelCalibrator::default(),
        }
    }

    pub fn is_detected(&self) -> bool {
        let detection_gap = self
            .get_liquid_gap()
            .expect("Movemap detector was used without liquid level calibration.");

        let Some(ref difference_mask) = self.last_difference_mask else {
            return false;
        };

        let difference_mask = &imageops::crop_imm(
            difference_mask,
            0,
            detection_gap.0,
            difference_mask.width(),
            detection_gap.1 - detection_gap.0,
        )
        .to_image();

        let abs_difference = get_abs_difference(difference_mask);

        if abs_difference
            > self
                .threshold
                .expect("Movemap detector was used without threshold calibration.")
        {
            info!("Detected {}/{}", abs_difference, self.threshold.unwrap());
            return true;
        };
        false
    }

    pub fn set_threshold(&mut self) -> bool {
        let Some(threshold) = self.threshold_calibrator.get() else {
            return false;
        };

        self.threshold = Some(
            (threshold as f32
                * (self
                    .settings
                    .read()
                    .unwrap()
                    .movemap
                    .calibration_multiplier_precents as f32
                    / 100.0)) as u32,
        );

        return true;
    }

    pub fn set_liquid_level(&mut self) -> bool {
        let Some(liquid_level) = self.liquid_level_calibrator.get() else {
            return false;
        };

        self.liquid_level = Some(liquid_level);

        return true;
    }

    pub fn update(&mut self, frame: RgbaImage) {
        let Some(ref last_frame) = self.last_frame else {
            self.last_frame = Some(frame);
            return;
        };

        if last_frame.width() == frame.width() && last_frame.height() == frame.height() {
            self.last_difference_mask = Some(rgba_difference_mask(last_frame, &frame));
        } else {
            self.last_difference_mask = None;
        }
       
        self.last_frame = Some(frame);
    }

    pub fn get_liquid_gap(&self) -> Option<(u32, u32)> {
        let Some(liquid_level) = self.liquid_level else {
            return None;
        };

        let capture_margin = self.settings.read().unwrap().capture.margin;

        let gap_size = self
            .settings
            .read()
            .unwrap()
            .movemap
            .detection_gap_size_precents;

        let liquid_level = liquid_level as i32;

        let gap_height = (capture_margin * 2) as f32 * (gap_size as f32 / 100.0);
        let gap = liquid_level - (gap_height.round() as i32);

        Some((std::cmp::max(gap, 0) as u32, liquid_level as u32))
    }

    pub fn reset(&mut self) {
        self.threshold_calibrator.thresholds.clear();
        self.liquid_level_calibrator.liquid_levels.clear();
        self.threshold = None;
        self.liquid_level = None;
        self.last_frame = None;
        self.last_difference_mask = None;
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct LiquidLevelCalibrator {
    pub liquid_levels: Vec<u32>,
}

impl LiquidLevelCalibrator {
    pub fn add(&mut self, difference_mask: &GrayImage) {
        let liquid_level = LiquidLevelCalibrator::get_liquid_level(difference_mask);
        if let Some(level) = liquid_level {
            self.liquid_levels.push(level);
        }
    }

    pub fn get(&self) -> Option<u32> {
        let count = self.liquid_levels.len() as u32;
        if count <= 3 {
            return None;
        };

        let mean: u32 = if count.is_multiple_of(2) {
            (self.liquid_levels[(count as f32 / 2f32) as usize]
                + self.liquid_levels[((count as f32 / 2f32) as usize) + 1])
                / 2
        } else {
            self.liquid_levels[(count as f32 / 2f32) as usize]
        };
        Some(mean)
    }

    fn get_liquid_level(difference_mask: &GrayImage) -> Option<u32> {
        let mut max_stack = 0;
        let mut max_stack_y: Option<u32> = None;
        for (i, row) in difference_mask.enumerate_rows() {
            let stack: u32 = row.map(|(_x, _y, pixel)| pixel.0[0] as u32).sum();
            if stack > max_stack {
                max_stack = stack;
                max_stack_y = Some(i);
            }
        }
        max_stack_y
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct ThresholdCalibrator {
    pub thresholds: Vec<u32>,
}

impl ThresholdCalibrator {
    pub fn add(&mut self, difference_mask: &GrayImage, liquid_gap: (u32, u32)) {
        let abs_difference = get_cropped_abs_difference(difference_mask, liquid_gap);
        self.thresholds.push(abs_difference);
    }

    pub fn get(&self) -> Option<u32> {
        self.thresholds.iter().max().cloned()
    }
}

pub fn rgba_difference_mask(img1: &RgbaImage, img2: &RgbaImage) -> GrayImage {
    assert_eq!(
        img1.dimensions(),
        img2.dimensions(),
        "Images must have the same dimensions"
    );

    let (width, height) = img1.dimensions();

    let mut mask = GrayImage::new(width, height);

    for (x, y, p1) in img1.enumerate_pixels() {
        let p2 = img2.get_pixel(x, y);

        let r_diff = (p1[0] as f32 - p2[0] as f32).powi(2);
        let g_diff = (p1[1] as f32 - p2[1] as f32).powi(2);
        let b_diff = (p1[2] as f32 - p2[2] as f32).powi(2);

        let distance = (r_diff + g_diff + b_diff).sqrt();
        let result = distance.min(255.0) as u8;

        mask.put_pixel(x, y, Luma([result]));
    }

    mask
}

pub fn get_abs_difference(difference_mask: &GrayImage) -> u32 {
    let abs_difference: u32 = difference_mask
        .as_raw()
        .iter()
        .map(|&pixel| pixel as u32)
        .sum();
    abs_difference
}

fn get_cropped_abs_difference(difference_mask: &GrayImage, liquid_gap: (u32, u32)) -> u32 {
    let width = difference_mask.width();
    let height = liquid_gap.1 - liquid_gap.0;

    let cropped_mask = imageops::crop_imm(difference_mask, 0, liquid_gap.0, width, height);
    let current_diff = get_abs_difference(&cropped_mask.to_image());
    return current_diff;
}
