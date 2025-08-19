#!/bin/bash

# Setup script for Phonon
# Downloads dependencies and samples

echo "🎵 Phonon Setup"
echo "===================="
echo ""

# Check for required tools
echo "📋 Checking dependencies..."

if ! command -v cargo &> /dev/null; then
    echo "  ⚠️  Rust not found. Please install: pkg install rust"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo "  ⚠️  Node.js not found. Please install: pkg install nodejs"
    exit 1
fi

if ! command -v git &> /dev/null; then
    echo "  ⚠️  Git not found. Please install: pkg install git"
    exit 1
fi

echo "  ✓ All dependencies found"
echo ""

# Download Dirt-Samples
if [ ! -d "dirt-samples" ]; then
    echo "📥 Downloading Dirt-Samples (389MB)..."
    git clone --depth 1 https://github.com/tidalcycles/Dirt-Samples.git dirt-samples
    echo "  ✓ Samples downloaded"
else
    echo "  ✓ Dirt-Samples already exists"
fi
echo ""

# Build Fermion
echo "🔨 Building Fermion synthesis engine..."
cd fermion && cargo build --release
if [ $? -eq 0 ]; then
    echo "  ✓ Fermion built successfully"
else
    echo "  ❌ Fermion build failed"
    exit 1
fi
cd ..
echo ""

# Install Node dependencies
echo "📦 Installing Node.js dependencies..."
cd boson && npm install
if [ $? -eq 0 ]; then
    echo "  ✓ Node dependencies installed"
else
    echo "  ❌ npm install failed"
    exit 1
fi
cd ..
echo ""

# Make scripts executable
chmod +x phonon
chmod +x get-samples.sh
chmod +x install-strudel.sh

echo "✅ Setup complete!"
echo ""
echo "🎶 To start Phonon:"
echo "   ./phonon start"
echo ""
echo "📝 Edit patterns.phonon to change the music!"
echo ""
echo "Sample usage:"
echo "  bd    - Kick drum"
echo "  sn    - Snare"
echo "  hh    - Hi-hat"
echo "  cp    - Clap"
echo "  See dirt-samples/ for all available samples"