# Example 5: Complete Mix
# Combining synthesis, samples, and effects

tempo: 0.5

# Simple tone
~tone: sine 440 * 0.1

# LFO for modulation
~lfo: sine 0.25 * 0.5 + 0.5

# Filtered bass
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8

# Drum samples
~drums: s "bd sn hh*4 cp"

# Mix everything
out: ~tone + ~bass * 0.3 + ~drums * 0.6
