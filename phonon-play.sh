#!/bin/bash
# Simple Phonon file player

# Build if needed
if [ ! -f "target/debug/examples/play_phonon" ]; then
    echo "Building play_phonon..."
    cargo build --example play_phonon
fi

if [ $# -eq 0 ]; then
    echo "Usage: $0 <file.phonon|dsl_code> [duration]"
    echo ""
    echo "Examples:"
    echo "  $0 examples/live_beat.phonon"
    echo "  $0 examples/live_beat.phonon 8"
    echo "  $0 -c '~kick: impulse 4 >> mul 100 >> lpf 80 0.9; out: ~kick'"
    exit 1
fi

# Run the player
cargo run --example play_phonon "$@"