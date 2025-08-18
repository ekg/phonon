#!/bin/bash

# Play Strudel patterns with Phonon Forge

echo "🎵 Phonon Forge - Strudel Player"
echo "================================"
echo ""

# Check dependencies
if ! command -v node &> /dev/null; then
    echo "⚠️  Node.js not found. To install:"
    echo "   pkg install nodejs"
    echo ""
fi

if ! command -v cargo &> /dev/null; then
    echo "⚠️  Rust not found. To install:"
    echo "   pkg install rust"
    echo ""
fi

echo "📋 Current pattern in patterns.phonon:"
echo "--------------------------------------"
head -26 patterns.phonon
echo ""

echo "🎼 This pattern includes:"
echo "  • Four-on-floor kick (bd*4)"
echo "  • Clap pattern with ghost notes"
echo "  • Euclidean hi-hat rhythm (7,16,2)"
echo "  • Open hat accents"
echo "  • Bass line in C minor (c2, eb2, g2)"
echo "  • Acid-style lead melody"
echo "  • Percussion (rimshot & cowbell)"
echo "  • Half-time feel with .slow(2)"
echo ""

echo "To play this pattern:"
echo "---------------------"
echo "1. Start Fermion (Rust synth):"
echo "   ./fermion/target/release/fermion serve"
echo ""
echo "2. Start Boson (Pattern engine) in another terminal:"
echo "   node boson/boson-strudel.js watch"
echo ""
echo "3. The pattern will start playing automatically!"
echo ""
echo "4. Edit patterns.phonon to change the music live"
echo ""

# If everything is available, offer to start
if command -v node &> /dev/null && [ -f "boson/boson-strudel.js" ]; then
    echo "Press Enter to start the pattern engine (Ctrl+C to stop)..."
    read
    
    echo "Starting Boson with Strudel patterns..."
    node boson/boson-strudel.js watch
fi