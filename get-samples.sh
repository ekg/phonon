#!/bin/bash

# Get Dirt-Samples for Phonon
# This gives us the exact same samples as Strudel/TidalCycles

echo "📦 Getting Dirt-Samples for Phonon..."
echo ""

# Check if git is available
if ! command -v git &> /dev/null; then
    echo "❌ git is not installed!"
    echo "Please install git first:"
    echo "  pkg install git"
    exit 1
fi

cd "$(dirname "$0")"

# Clone the Dirt-Samples repository
if [ -d "dirt-samples" ]; then
    echo "✓ dirt-samples directory already exists"
    echo "  To update: cd dirt-samples && git pull"
else
    echo "📥 Cloning Dirt-Samples repository..."
    git clone --depth 1 https://github.com/tidalcycles/Dirt-Samples.git dirt-samples
    
    if [ $? -eq 0 ]; then
        echo "✅ Successfully downloaded Dirt-Samples!"
    else
        echo "❌ Failed to clone repository"
        exit 1
    fi
fi

# Create symlink for easier access
if [ ! -L "samples" ]; then
    ln -s dirt-samples samples
    echo "✓ Created 'samples' symlink"
fi

echo ""
echo "📝 Sample Usage (Strudel/Tidal syntax):"
echo ""
echo "  bd        → plays dirt-samples/bd/BT0A0A7.wav (first file)"
echo "  bd:1      → plays dirt-samples/bd/BT0AAD0.wav (second file)"
echo "  sn:2      → plays third snare sample"
echo "  hh:0      → plays first hihat (same as 'hh')"
echo ""
echo "Available sample folders:"
ls -d dirt-samples/*/ 2>/dev/null | head -20 | xargs -n1 basename | sed 's/^/  /'
echo ""
echo "✨ Ready to use with Phonon!"