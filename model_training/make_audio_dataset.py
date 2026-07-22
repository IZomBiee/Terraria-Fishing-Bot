import os
import wave
import glob

# ==================== CONFIGURATION SETTINGS ====================
INPUT_DIR = r"A:\Datasets\Terraria-Sound"                 # Directory containing your raw .wav and .txt files
OUTPUT_DIR = r"dataset"                                  # Where the positive/negative folders will be created
FRAGMENT_DURATION = 0.7                                  # The EXACT duration (in seconds) your model requires
BUFFER_MARGIN = 0.5                                      # Minimum distance (in seconds) between positive labels and background samples
# ================================================================

pos_dir = os.path.join(OUTPUT_DIR, "positive")
neg_dir = os.path.join(OUTPUT_DIR, "negative")
os.makedirs(pos_dir, exist_ok=True)
os.makedirs(neg_dir, exist_ok=True)

def slice_wav(source_wav, start_time, end_time, output_path):
    params = source_wav.getparams()
    frame_rate = params.framerate
    
    start_frame = int(start_time * frame_rate)
    end_frame = int(end_time * frame_rate)
    num_frames = end_frame - start_frame
    
    if num_frames <= 0:
        return
        
    source_wav.setpos(start_frame)
    audio_data = source_wav.readframes(num_frames)
    
    with wave.open(output_path, 'wb') as out_file:
        out_file.setparams(params)
        out_file.writeframes(audio_data)

# Scan for WAV files
search_path = os.path.join(INPUT_DIR, "*.wav")
wav_files = glob.glob(search_path)

if not wav_files:
    print(f"No .wav files found in target directory: {INPUT_DIR}")
    exit()

for wav_path in wav_files:
    biome_name = os.path.splitext(os.path.basename(wav_path))[0]
    txt_path = os.path.join(INPUT_DIR, f"{biome_name}.txt")
    
    if not os.path.exists(txt_path):
        print(f"Skipping [{os.path.basename(wav_path)}]: Missing matching label file '{os.path.basename(txt_path)}'")
        continue
        
    print(f"Processing biome: {biome_name}...")
    
    with wave.open(wav_path, 'rb') as src:
        frame_rate = src.getframerate()
        total_duration = src.getnframes() / frame_rate
        
        # 1. Parse and recalculate fixed-size positive intervals
        adjusted_positives = []
        raw_intervals = []
        
        with open(txt_path, 'r') as f:
            for line in f:
                if not line.strip(): 
                    continue
                parts = line.replace('\t', ' ').split(maxsplit=2)
                if len(parts) >= 2:
                    raw_intervals.append((float(parts[0]), float(parts[1])))
        
        for idx, (start, end) in enumerate(raw_intervals):
            # Find the center point of the original user marker
            center = (start + end) / 2.0
            
            # Expand outward from center to hit the exact target duration
            p_start = center - (FRAGMENT_DURATION / 2.0)
            p_end = center + (FRAGMENT_DURATION / 2.0)
            
            # Guard against edge-of-file boundary crashes
            if p_start < 0:
                p_start = 0.0
                p_end = FRAGMENT_DURATION
            if p_end > total_duration:
                p_end = total_duration
                p_start = max(0.0, total_duration - FRAGMENT_DURATION)
                
            adjusted_positives.append((p_start, p_end))
            
            # Save exact positive slice
            out_path = os.path.join(pos_dir, f"{biome_name}_shake_{idx}.wav")
            slice_wav(src, p_start, p_end, out_path)
            
        # Sort adjusted items to cleanly parse out negative background blocks
        adjusted_positives.sort()
        
        # 2. Slice Negative Blocks with Buffer Distance
        last_end = 0.0
        neg_idx = 0
        
        for p_start, p_end in adjusted_positives:
            # We can only harvest negative samples up to (p_start - BUFFER_MARGIN)
            safe_limit = p_start - BUFFER_MARGIN
            
            while safe_limit - last_end >= FRAGMENT_DURATION:
                neg_end = last_end + FRAGMENT_DURATION
                out_path = os.path.join(neg_dir, f"{biome_name}_bg_{neg_idx}.wav")
                slice_wav(src, last_end, neg_end, out_path)
                neg_idx += 1
                last_end = neg_end
                
            # Jump past the positive sound block AND add the tail buffer
            last_end = max(last_end, p_end + BUFFER_MARGIN)
                
        # Handle trailing end of file dead space
        while total_duration - last_end >= FRAGMENT_DURATION:
            neg_end = last_end + FRAGMENT_DURATION
            out_path = os.path.join(neg_dir, f"{biome_name}_bg_{neg_idx}.wav")
            slice_wav(src, last_end, neg_end, out_path)
            neg_idx += 1
            last_end = neg_end

print(f"\nSuccess! Fixed {FRAGMENT_DURATION}s windows generated with {BUFFER_MARGIN}s safety margin in '{OUTPUT_DIR}'")