# Simple Melody - A working example with clear output
# Render with: phonon render simple_melody.phonon simple_melody.wav --duration 8

# Simple repeating melody
melody = sine "440 550 660 550" # lpf 2000 2

# Bass line
bass = saw "110 110 82.5 110" # lpf 500 3

# Simple percussion (filtered noise)
perc = noise # lpf("100 100 100 5000", 10)

# Output - each part individually works
# Comment/uncomment to hear different parts

# Melody only
out melody * 0.2

# Bass only
# out bass * 0.2

# Percussion only
# out perc * 0.15