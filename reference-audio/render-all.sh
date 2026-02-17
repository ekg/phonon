#!/bin/bash

# Render all golden reference audio files
# Usage: ./render-all.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Duration for each pattern (8 seconds = multiple cycles at all tempos)
DURATION=8.0

# Find phonon binary
if command -v phonon &> /dev/null; then
    PHONON=phonon
elif [ -f "../target/release/phonon" ]; then
    PHONON="../target/release/phonon"
else
    echo "Error: phonon binary not found"
    echo "Run 'cargo build --release --bin phonon' first"
    exit 1
fi

echo "🎵 Rendering golden reference audio files..."
echo "Using: $PHONON"
echo ""

# Counter for stats
RENDERED=0
FAILED=0

# Find all .phonon files and render them
for phonon_file in $(find . -name "*.phonon" -type f | sort); do
    wav_file="${phonon_file%.phonon}.wav"
    echo -n "Rendering: $phonon_file -> $wav_file ... "

    if $PHONON render "$phonon_file" "$wav_file" -d $DURATION > /dev/null 2>&1; then
        # Get RMS level from the render
        rms=$($PHONON render "$phonon_file" "$wav_file" -d $DURATION 2>&1 | grep "RMS level" | awk '{print $3}')
        echo "✅ (RMS: $rms)"
        ((RENDERED++))
    else
        echo "❌ Failed"
        ((FAILED++))
    fi
done

echo ""
echo "=========================================="
echo "Rendered: $RENDERED files"
if [ $FAILED -gt 0 ]; then
    echo "Failed:   $FAILED files"
fi
echo "=========================================="
