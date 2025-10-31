#!/usr/bin/env python3
"""Debug onset detection issues"""

import wave
import struct
import math

def read_wav(path):
    """Read WAV file and return samples as list of floats"""
    with wave.open(path, 'r') as wav:
        sample_rate = wav.getframerate()
        num_frames = wav.getnframes()

        # Read all frames
        frames = wav.readframes(num_frames)

        # Convert to floats
        if wav.getsampwidth() == 2:
            # 16-bit PCM
            samples = struct.unpack(f'<{num_frames}h', frames)
            samples = [s / 32768.0 for s in samples]
        elif wav.getsampwidth() == 4:
            # 32-bit float
            samples = struct.unpack(f'<{num_frames}f', frames)
        else:
            raise ValueError(f"Unsupported sample width: {wav.getsampwidth()}")

        return samples, sample_rate

def calculate_energy_envelope(samples, sample_rate, window_ms=5.0):
    """Calculate RMS energy envelope"""
    window_samples = int((sample_rate * window_ms) / 1000.0)

    envelope = []
    for i in range(0, len(samples), window_samples):
        chunk = samples[i:i+window_samples]
        if len(chunk) == 0:
            break
        rms = math.sqrt(sum(s*s for s in chunk) / len(chunk))
        time = i / sample_rate
        envelope.append((time, rms))

    return envelope

def detect_onsets(envelope, threshold_factor=0.15, min_distance_ms=60):
    """Detect onsets using threshold and minimum distance"""
    if not envelope:
        return []

    max_energy = max(e[1] for e in envelope)
    threshold = max_energy * threshold_factor

    onsets = []
    last_onset_time = -1.0
    min_distance_s = min_distance_ms / 1000.0

    for time, energy in envelope:
        if energy > threshold and (time - last_onset_time) > min_distance_s:
            onsets.append((time, energy))
            last_onset_time = time

    return onsets

def main():
    wav_path = "/tmp/test_eight_steps.wav"

    print(f"Analyzing: {wav_path}")
    print("="*60)

    samples, sample_rate = read_wav(wav_path)

    print(f"Sample rate: {sample_rate} Hz")
    print(f"Duration: {len(samples) / sample_rate:.3f}s")
    print(f"Num samples: {len(samples)}")
    print()

    # Calculate envelope
    envelope = calculate_energy_envelope(samples, sample_rate, window_ms=5.0)

    max_energy = max(e[1] for e in envelope)
    print(f"Max energy: {max_energy:.6f}")
    print()

    # Try different thresholds
    for thresh_factor in [0.05, 0.10, 0.15, 0.20, 0.25, 0.30]:
        onsets = detect_onsets(envelope, threshold_factor=thresh_factor, min_distance_ms=60)
        print(f"Threshold {thresh_factor*100:.0f}% of max ({max_energy * thresh_factor:.6f}):")
        print(f"  Detected {len(onsets)} onsets")
        if onsets and len(onsets) <= 12:
            for i, (time, energy) in enumerate(onsets):
                print(f"    Onset {i+1}: {time:.3f}s (energy: {energy:.6f})")
        print()

    # Show energy envelope peaks
    print("Energy envelope (top 20 peaks):")
    sorted_env = sorted(envelope, key=lambda x: x[1], reverse=True)[:20]
    for i, (time, energy) in enumerate(sorted_env):
        print(f"  {i+1:2d}. {time:.3f}s: {energy:.6f}")

if __name__ == "__main__":
    main()
