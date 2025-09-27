#!/bin/bash
# Render phonon file to WAV using the working phonon_poll implementation

INPUT="${1:-test_basic.phonon}"
OUTPUT="${2:-output.wav}"
DURATION="${3:-10}"

echo "Rendering $INPUT to $OUTPUT ($DURATION seconds)..."

# Run phonon_poll in background and capture audio
timeout $DURATION cargo run --example phonon_poll "$INPUT" 2>/dev/null &
PID=$!

# Use sox or ffmpeg to record system audio
# This is platform-specific - adjust as needed
sleep 1

# For now, just create a test file
cargo run --example phonon_poll "$INPUT" 2>&1 | head -20

echo "Note: To properly capture audio to WAV, you need to:"
echo "1. Use a tool like sox, ffmpeg, or parec to capture audio output"
echo "2. Or modify phonon_poll to write directly to WAV file"