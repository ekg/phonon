# Parameter Patterns Demo
# Demonstrates gain, pan, and speed pattern modulation using Tidal-style syntax

tempo 2.0  # 120 BPM

# GAIN PATTERNS (Dynamics)
# Uncomment one at a time to explore

# Decreasing velocity (like a drum machine)
# out: s("bd*4") # gain("1.0 0.8 0.6 0.4")

# Accent pattern (like TB-303)
# out: s("bd*8") # gain("1.0 0.3 0.6 0.3 1.0 0.3 0.6 0.3")

# Ghost notes on hi-hats
# out: s("hh*16") # gain("0.8 0.3 0.6 0.3 0.8 0.3 0.6 0.3 0.8 0.3 0.6 0.3 0.8 0.3 0.6 0.3")

# PAN PATTERNS (Stereo)
# Uncomment one at a time

# Ping-pong effect
# out: s("hh*8") # gain(0.8) # pan("-1 1 -1 1 -1 1 -1 1")

# Sweep left to right
# out: s("bd*4") # gain(1.0) # pan("-1 -0.5 0.5 1")

# Wide stereo hi-hats
# out: s("hh*8") # gain(0.6) # pan("-1 1")

# SPEED PATTERNS (Pitch)
# Uncomment one at a time

# Varying speeds (pitch modulation)
# out: s("bd*4") # gain(1.0) # speed("1.0 1.2 0.8 1.5")

# Octave jumps
# out: s("bd*2") # gain(1.0) # speed("1.0 2.0")

# Reverse samples with negative speed
# out: s("sn*2") # gain(1.0) # speed("1.0 -1.0")

# COMBINED PATTERNS
# Full control over dynamics, stereo, and pitch using Tidal-style chaining

# Expressive kick pattern - chain multiple modifiers with #
~kick: s("bd*8") # gain("1.0 0.7 0.8 0.6 1.0 0.7 0.8 0.6") # pan("-1 -0.5 0 0.5 1 0.5 0 -0.5") # speed("1.0 1.1 0.9 1.2 1.0 1.1 0.9 0.8")

# Hi-hats with dynamics and stereo
~hats: s("hh*16") # gain("0.7 0.4 0.6 0.3") # pan("-1 1 -0.5 0.5")

# Combine both
out: (~kick + ~hats) * 0.7

# Try modifying the pattern strings!
# Each parameter cycles through its values independently.
# Use # to chain modifiers in Tidal style.
