# Terraria Fishing Bot

Terraria advanced fishing bot, written entirely in Rust. Created for practicing/learning Rust.

## Features

- Reliable fishing in water/lava/honey
- 3 detection methods:
  - Audio (ONNX Neural Network model)
  - OCR (Text detection)
  - Movemap (Pixel movement tracking)
- Reliable fishing with noise like enemies
- GUI interface
- Automatic potion use
- Working with sonar potion
- Saving settings
- [ ] Fast unload to nearby chests
- [ ] Handle events (Blood Moon, Invasions)

## Preview

### Audio method

![Audio method preview](assets/audio_preview.png)

### OCR method

![OCR method preview](assets/ocr_preview.gif)

### Movemap method

![Movemap method preview](assets/movemap_preview.gif)

## Manual

1. Download the executable file from the releases tab in this repository.
3. Run the executable file.
4. Configure settings in the GUI and select your preferred detection method.
5. Set your cursor slightly below where you want to fish.
6. Press P to start/stop. Do not move the cursor or your character while active.

## Things to Know

### General
- The bot tracks the area around your cursor.
- The sound capture from the entire computer, not just the game.

### OCR Method
- Requires Sonar Potions to detect fish text overlays.
- It works inconsistently because it uses a generic model. In the future, there will be a specialized one that will work better.

### Movemap Method
- The bot WILL NOT WORK in movemap mode if you do not have waves enabled in Terraria settings.
- The bot may not work properly in the Underworld due to heat distortion. Setting a contrasting background can help.
- During heavy rain, false alarms may occur. Building a roof overhead prevents this.
- While fishing in lava, increase the gap size, threshold, and noise delay settings to account for lava bubbles.