use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::time::Instant;
use strum_macros::AsRefStr;

#[derive(Debug, PartialEq, AsRefStr, Clone)]
pub enum BotState {
    Idle,
    LiquidLevelCalibration(Instant),
    MovemapThresholdCalibration(Instant),
    WaitingForBite(Instant),
    Cast,
    Catch,
    CastingCooldown(Instant),
    CatchingCooldown(Instant),
}

impl Display for BotState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            BotState::Idle => "Idle",
            BotState::LiquidLevelCalibration(..) => "LiquidLevelCalibration",
            BotState::MovemapThresholdCalibration(..) => "ThresholdCalibration",
            BotState::WaitingForBite(..) => "WaitingForBite",
            BotState::Cast => "Cast",
            BotState::Catch => "Catch",
            BotState::CastingCooldown(..) => "CastingCooldown",
            BotState::CatchingCooldown(..) => "CatchingCooldown",
        };
        write!(f, "{}", name)
    }
}

pub enum BotCommand {
    Start,
    Stop,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum DetectionMethod {
    Movemap,
    Audio,
    Ocr,
    Yolo,
    YoloAudio,
}
