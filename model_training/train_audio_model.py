import os
import sys
import glob
import random
import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader
import torchaudio
import torchaudio.transforms as T

# --- FFMPEG RUNTIME ENGINE FIX FOR WINDOWS ---
FFMPEG_BIN_DIR = r"C:\Program Files\ffmpeg-8.1.2-full_build-shared\bin"
if os.path.exists(FFMPEG_BIN_DIR):
    os.add_dll_directory(FFMPEG_BIN_DIR)

# --- CONFIGURATION ---
TARGET_SR = 22050
DURATION = 0.7
NUM_SAMPLES = int(TARGET_SR * DURATION)
BATCH_SIZE = 16              # FIXED: Small mini-batches force multiple weight updates per epoch
EPOCHS = 20                  
LEARNING_RATE = 0.001        # FIXED: Standard safe step-size for mini-batch Adam
DATASET_PATH = r"C:\Users\patri\Desktop\terraria_fishing_bot\model_training\dataset"

N_MELS = 64*4
N_FFT = 1024*4
HOP_LENGTH = 512*4

if not torch.cuda.is_available():
    sys.exit("Error: CUDA is not available.")
device = torch.device("cuda")
print("CUDA Initialized.")

# --- 1. DATASET PROCESSING ---
class AudioDataset(Dataset):
    def __init__(self, file_paths, labels, target_sr, num_samples, n_mels, n_fft, hop_length, is_train=True):
        self.file_paths = file_paths
        self.labels = labels
        self.target_sr = target_sr
        self.num_samples = num_samples
        self.is_train = is_train
        
        self.mel_spectrogram = T.MelSpectrogram(
            sample_rate=self.target_sr, n_fft=n_fft, hop_length=hop_length, n_mels=n_mels
        )
        # FIXED: Use standard stable power-to-DB mapping transform
        self.amplitude_to_db = T.AmplitudeToDB(stype="power")

    def __len__(self):
        return len(self.file_paths)

    def __getitem__(self, index):
        path = self.file_paths[index]
        label = self.labels[index]
        
        signal, sr = torchaudio.load(path)
        
        if signal.shape[0] > 1:
            signal = torch.mean(signal, dim=0, keepdim=True)
            
        if signal.shape[1] > self.num_samples:
            signal = signal[:, :self.num_samples]
        elif signal.shape[1] < self.num_samples:
            pad_amount = self.num_samples - signal.shape[1]
            signal = torch.nn.functional.pad(signal, (0, pad_amount))
            
        if self.is_train and random.random() > 0.5:
            noise = torch.randn_like(signal) * 0.002
            signal = signal + noise
        
        mel = self.mel_spectrogram(signal)
        mel_db = self.amplitude_to_db(mel)
        
        return mel_db, torch.tensor(label, dtype=torch.float32)

# --- 2. DYNAMIC CNN ARCHITECTURE ---
class AudioClassifier(nn.Module):
    def __init__(self, input_shape):
        super().__init__()
        self.conv1 = nn.Sequential(
            nn.Conv2d(1, 16, kernel_size=3, stride=1, padding=1),
            nn.BatchNorm2d(16), 
            nn.ReLU(),
            nn.MaxPool2d(kernel_size=2)
        )
        self.conv2 = nn.Sequential(
            nn.Conv2d(16, 32, kernel_size=3, stride=1, padding=1),
            nn.BatchNorm2d(32),
            nn.ReLU(),
            nn.MaxPool2d(kernel_size=2)
        )
        self.flatten = nn.Flatten()
        
        with torch.no_grad():
            dummy_x = torch.zeros(1, *input_shape) 
            dummy_x = self.conv2(self.conv1(dummy_x))
            flat_features = self.flatten(dummy_x).shape[1]
        
        self.fc = nn.Sequential(
            nn.Linear(flat_features, 64), 
            nn.ReLU(),
            nn.Dropout(0.2),  
            nn.Linear(64, 1) # FIXED: Removed Sigmoid. BCEWithLogitsLoss processes raw logits directly.
        )

    def forward(self, x):
        x = self.conv1(x)
        x = self.conv2(x)
        x = self.flatten(x)
        return self.fc(x).squeeze(1)

# --- 3. TRAINING & VALIDATION PIPELINE ---
def main():
    all_paths = []
    all_labels = []
    
    pos_files = glob.glob(os.path.join(DATASET_PATH, "positive", "*.wav"))
    for f in pos_files:
        all_paths.append(f)
        all_labels.append(1.0)
        
    neg_files = glob.glob(os.path.join(DATASET_PATH, "negative", "*.wav"))
    for f in neg_files:
        all_paths.append(f)
        all_labels.append(0.0)
        
    total_samples = len(all_paths)
    print(f"Total files discovered: {total_samples}")
    if total_samples == 0:
        return

    combined = list(zip(all_paths, all_labels))
    random.seed(42)
    random.shuffle(combined)
    all_paths, all_labels = zip(*combined)

    train_size = int(0.8 * total_samples)
    train_paths, train_labels = all_paths[:train_size], all_labels[:train_size]
    val_paths, val_labels = all_paths[train_size:], all_labels[train_size:]

    # Calculate class distribution to fight data imbalance
    num_pos = sum(train_labels)
    num_neg = len(train_labels) - num_pos
    # Scaling factor: heavily punishes missing a rare positive bite sequence
    pos_weight_val = num_neg / max(num_pos, 1.0)
    pos_weight = torch.tensor([pos_weight_val]).to(device)
    print(f"Class Distribution -> Positives: {int(num_pos)}, Negatives: {int(num_neg)} | Imbalance Weight: {pos_weight_val:.2f}x")

    train_dataset = AudioDataset(train_paths, train_labels, TARGET_SR, NUM_SAMPLES, N_MELS, N_FFT, HOP_LENGTH, is_train=True)
    val_dataset = AudioDataset(val_paths, val_labels, TARGET_SR, NUM_SAMPLES, N_MELS, N_FFT, HOP_LENGTH, is_train=False)
    
    sample_mel, _ = train_dataset[0]
    input_shape = sample_mel.shape
    print(f"Dynamic Spectrogram Input Shape: {list(input_shape)}")

    train_loader = DataLoader(train_dataset, batch_size=BATCH_SIZE, shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=BATCH_SIZE, shuffle=False)
    
    model = AudioClassifier(input_shape=input_shape).to(device)
    
    # FIXED: Using BCEWithLogitsLoss combined with automatic balance weighting scales loss evenly
    criterion = nn.BCEWithLogitsLoss(pos_weight=pos_weight)
    optimizer = optim.Adam(model.parameters(), lr=LEARNING_RATE, weight_decay=1e-4)
    
    print(f"Training on {len(train_dataset)} samples, validating on {len(val_dataset)} samples...\n")
    
    for epoch in range(EPOCHS):
        model.train()
        correct, total = 0, 0
        for batch_images, batch_labels in train_loader:
            batch_images, batch_labels = batch_images.to(device), batch_labels.to(device)
            outputs = model(batch_images)
            loss = criterion(outputs, batch_labels)
            
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            
            # Since model outputs logits, positive classification maps to >= 0.0 instead of 0.5
            preds = (outputs >= 0.0).float()
            correct += (preds == batch_labels).sum().item()
            total += batch_labels.size(0)
            
        train_acc = (correct / total) * 100
        
        model.eval()
        val_correct, val_total = 0, 0
        true_positives, false_positives, false_negatives = 0, 0, 0
        
        with torch.no_grad():
            for batch_images, batch_labels in val_loader:
                batch_images, batch_labels = batch_images.to(device), batch_labels.to(device)
                outputs = model(batch_images)
                preds = (outputs >= 0.0).float()
                
                val_correct += (preds == batch_labels).sum().item()
                val_total += batch_labels.size(0)
                
                # Calculate True Positives, False Positives, False Negatives
                true_positives += ((preds == 1.0) & (batch_labels == 1.0)).sum().item()
                false_positives += ((preds == 1.0) & (batch_labels == 0.0)).sum().item()
                false_negatives += ((preds == 0.0) & (batch_labels == 1.0)).sum().item()
                
        val_acc = (val_correct / val_total) * 100
        
        # Calculate Precision and Recall safely
        precision = true_positives / (true_positives + false_positives) if (true_positives + false_positives) > 0 else 0.0
        recall = true_positives / (true_positives + false_negatives) if (true_positives + false_negatives) > 0 else 0.0
        f1_score = 2 * (precision * recall) / (precision + recall) if (precision + recall) > 0 else 0.0
        
        print(f"Epoch [{epoch+1:02d}/{EPOCHS}] -> Train Acc: {train_acc:.2f}% | Val Acc: {val_acc:.2f}% | F1-Score: {f1_score:.2f} (P: {precision:.2f}, R: {recall:.2f})")
        
    # --- EXPORT TO ONNX FOR RUST INTEGRATION ---
    print("\nExporting model to ONNX for Rust...")
    
    # Structural Wrapper: Appends a Sigmoid layer strictly for export deployment 
    # so your Rust application continues to receive clean 0.0 to 1.0 probability float calculations!
    class ONNXExportWrapper(nn.Module):
        def __init__(self, trained_model):
            super().__init__()
            self.base_model = trained_model
            self.sigmoid = nn.Sigmoid()
        def forward(self, x):
            return self.sigmoid(self.base_model(x))

    export_ready_model = ONNXExportWrapper(model).to(device)
    export_ready_model.eval()
    
    dummy_input = torch.randn(1, *input_shape).to(device)
    batch_dim = torch.export.Dim("batch_size")
    
    torch.onnx.export(
        export_ready_model, 
        dummy_input, 
        "sound_detector.onnx", 
        export_params=True, 
        opset_version=18, 
        input_names=['input'],   
        output_names=['output'], 
        dynamic_shapes={'x': {0: batch_dim}},
    )
    print("Successfully generated stable 'sound_detector.onnx'!")

if __name__ == "__main__":
    main()