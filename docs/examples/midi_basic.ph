# Basic MIDI Recording Example
#
# Instructions:
# 1. cargo run --release --bin phonon -- edit
# 2. Alt+M to connect MIDI device
# 3. Alt+R to start recording
# 4. Play: C4, E4, G4, C5
# 5. Alt+R to stop
# 6. Alt+I to insert notes, Alt+V to insert velocities

tempo: 2.0

# After recording, insert patterns here:
# ~notes: n "c4 e4 g4 c5"
# ~vel: "0.8 1.0 0.7 0.9"

# Play with recorded velocity
# out: ~notes # gain ~vel
