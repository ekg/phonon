# MIDI Dynamics Example
#
# Demonstrates velocity-controlled gain for expressive performance

tempo: 2.0

# Example recorded pattern (record your own with Alt+R, Alt+I, Alt+V):
~melody: n "c4 d4 e4 f4 g4 a4 g4 f4"
~dynamics: "0.5 0.6 0.7 0.8 1.0 0.9 0.7 0.5"

# Apply dynamics to melody
~expressive: ~melody # gain ~dynamics

# Add envelope for more natural sound
~shaped: ~expressive # adsr 0.01 0.1 0.7 0.2

out: ~shaped * 0.8
