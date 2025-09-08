#!/bin/bash
# Phonon live coding launcher

# Build if needed
if [ ! -f "target/debug/examples/live_phonon" ]; then
    echo "Building live_phonon..."
    cargo build --example live_phonon
fi

# Default to live_session.phonon if no file specified
FILE="${1:-examples/live_session.phonon}"
DURATION="${2:-4}"

# Create file if it doesn't exist
if [ ! -f "$FILE" ]; then
    echo "Creating $FILE..."
    cat > "$FILE" << 'EOF'
# Phonon Live Session
# Edit and save to hear changes!

~kick: impulse 4 >> mul 80 >> lpf 100 0.9
~hihat: impulse 8 >> mul 10 >> noise >> mul 0.2 >> hpf 8000 0.9
out: ~kick + ~hihat >> mul 0.8
EOF
fi

echo "🎵 Starting Phonon live coding session"
echo "📝 Edit: $FILE"
echo "⏱️  Duration: ${DURATION}s per loop"
echo ""

# Run the live coder
cargo run --example live_phonon "$FILE" "$DURATION"