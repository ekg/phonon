#!/bin/bash

# Play Phonon demo patterns

echo "🎵 Phonon Demo Player"
echo "=========================="
echo ""
echo "Available demos:"
echo "  1) House    - Four-on-floor Chicago house"
echo "  2) Techno   - Berlin-style driving techno"
echo "  3) Hip Hop  - Classic boom bap beat"
echo "  4) Jazz     - Bebop swing pattern"
echo "  5) Afrobeat - Fela Kuti polyrhythms"
echo "  6) DnB      - Drum & Bass/Jungle"
echo "  7) Reggae   - One drop rhythm"
echo "  8) Bossa    - Brazilian bossa nova"
echo "  9) Ambient  - Ethereal soundscape"
echo " 10) Rock     - Classic rock beat"
echo ""
echo -n "Select demo (1-10): "
read choice

case $choice in
    1) pattern="house" ;;
    2) pattern="techno" ;;
    3) pattern="hiphop" ;;
    4) pattern="jazz" ;;
    5) pattern="afrobeat" ;;
    6) pattern="dnb" ;;
    7) pattern="reggae" ;;
    8) pattern="bossa" ;;
    9) pattern="ambient" ;;
    10) pattern="rock" ;;
    *) echo "Invalid choice!"; exit 1 ;;
esac

echo ""
echo "Loading $pattern pattern..."
cp demos/$pattern.phonon patterns.phonon
cp demos/$pattern.phonon boson/patterns.phonon

echo "▶️  Playing $pattern beat..."
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Start the system
./phonon start