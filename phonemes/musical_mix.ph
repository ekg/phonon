-- Musical composition with bass, lead, and chords
bass = saw "55 55 82.5 55" # lpf("500 800 1000 500", 3)
lead = square "220 330 440 330" # lpf 2000 2
pad = saw 110 # lpf 800 2

-- Full mix
out bass * 0.3 + lead * 0.15 + pad * 0.1
