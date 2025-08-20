#!/usr/bin/env python3
"""
Play a 10-second house beat using Python
Classic 4-on-the-floor pattern at 120 BPM
"""

import numpy as np
import struct
import subprocess
import time
from pathlib import Path

# Audio parameters
SAMPLE_RATE = 44100
TEMPO = 120  # BPM
BEAT_DURATION = 60.0 / TEMPO  # Duration of one beat in seconds
PATTERN_LENGTH = 16  # 16 steps (one bar)

def generate_kick():
    """Generate a kick drum sound"""
    duration = 0.2
    t = np.linspace(0, duration, int(SAMPLE_RATE * duration))
    # Low frequency sine with pitch envelope
    freq_env = 60 * (1 + np.exp(-t * 50))
    amplitude_env = np.exp(-t * 20)
    kick = np.sin(2 * np.pi * freq_env * t) * amplitude_env
    return kick

def generate_snare():
    """Generate a snare drum sound"""
    duration = 0.15
    t = np.linspace(0, duration, int(SAMPLE_RATE * duration))
    # Mix of noise and tone
    noise = np.random.normal(0, 0.1, len(t))
    tone = np.sin(2 * np.pi * 200 * t) * 0.3
    amplitude_env = np.exp(-t * 30)
    snare = (noise + tone) * amplitude_env
    return snare

def generate_hihat():
    """Generate a hi-hat sound"""
    duration = 0.05
    t = np.linspace(0, duration, int(SAMPLE_RATE * duration))
    # High frequency noise
    noise = np.random.normal(0, 0.2, len(t))
    amplitude_env = np.exp(-t * 100)
    hihat = noise * amplitude_env
    return hihat

def write_wav(filename, audio_data, sample_rate=44100):
    """Write audio data to WAV file"""
    # Normalize and convert to 16-bit
    audio_data = np.clip(audio_data, -1, 1)
    audio_16bit = (audio_data * 32767).astype(np.int16)
    
    with open(filename, 'wb') as f:
        # WAV header
        f.write(b'RIFF')
        f.write(struct.pack('<I', 36 + len(audio_16bit) * 2))
        f.write(b'WAVE')
        f.write(b'fmt ')
        f.write(struct.pack('<I', 16))  # fmt chunk size
        f.write(struct.pack('<H', 1))   # PCM
        f.write(struct.pack('<H', 1))   # Mono
        f.write(struct.pack('<I', sample_rate))
        f.write(struct.pack('<I', sample_rate * 2))
        f.write(struct.pack('<H', 2))   # Block align
        f.write(struct.pack('<H', 16))  # Bits per sample
        f.write(b'data')
        f.write(struct.pack('<I', len(audio_16bit) * 2))
        f.write(audio_16bit.tobytes())

def create_house_beat(duration_seconds=10):
    """Create a house beat pattern"""
    # House pattern: "bd bd bd bd sn ~ sn ~ hh ~ hh ~ hh ~ hh ~"
    pattern = [
        'bd', 'bd', 'bd', 'bd',  # Four-on-the-floor kick
        'sn', None, 'sn', None,   # Snare on 5 and 7
        'hh', None, 'hh', None,   # Off-beat hi-hats
        'hh', None, 'hh', None
    ]
    
    # Generate samples
    samples = {
        'bd': generate_kick(),
        'sn': generate_snare(),
        'hh': generate_hihat()
    }
    
    # Calculate step duration (16th notes at 120 BPM)
    step_duration = BEAT_DURATION / 4  # Quarter beat divided by 4 = 16th note
    samples_per_step = int(SAMPLE_RATE * step_duration)
    
    # Calculate total samples needed
    total_samples = int(SAMPLE_RATE * duration_seconds)
    
    # Create output buffer
    output = np.zeros(total_samples)
    
    # Fill the buffer with the pattern
    step_index = 0
    sample_pos = 0
    
    while sample_pos < total_samples:
        # Get current step in pattern
        sound = pattern[step_index % len(pattern)]
        
        if sound and sound in samples:
            # Add the sample to output
            sample_data = samples[sound]
            end_pos = min(sample_pos + len(sample_data), total_samples)
            copy_length = end_pos - sample_pos
            output[sample_pos:end_pos] += sample_data[:copy_length] * 0.5
        
        # Move to next step
        sample_pos += samples_per_step
        step_index += 1
    
    return output

def main():
    print("🎵 Phonon - House Beat Generator")
    print("📀 Creating 10-second house beat at 120 BPM...")
    
    # Generate the beat
    audio = create_house_beat(10)
    
    # Write to WAV file
    output_file = Path("/tmp/house_beat.wav")
    write_wav(output_file, audio)
    
    print(f"✅ House beat saved to {output_file}")
    print("▶️  Playing house beat...")
    
    # Try to play with mplayer
    try:
        subprocess.run(['mplayer', '-really-quiet', str(output_file)], 
                      capture_output=True, timeout=11)
        print("✅ Playback complete!")
    except FileNotFoundError:
        print("⚠️  mplayer not found. WAV file saved at:", output_file)
    except subprocess.TimeoutExpired:
        print("✅ Playback complete!")
    except Exception as e:
        print(f"⚠️  Could not play audio: {e}")
        print(f"📁 WAV file saved at: {output_file}")

if __name__ == "__main__":
    main()