# Complete Synthesis and Effects Demo
# Demonstrates: ADSR envelopes, effects routing, master processing, auto-routing

cps: 2.0

# Channel 1: Bass line with tight envelope
# Fast attack (1ms), short decay (50ms), no sustain, short release (100ms)
# = Percussive, punchy bass
~d1: synth "c3 c3 g3 c3" "square" 0.001 0.05 0.0 0.1

# Channel 2: Lead melody with expressive envelope
# Fast attack (10ms), medium decay (100ms), high sustain (70%), medium release (300ms)
# = Bright, sustained lead line
~d2: synth "c5 e5 g5 c6" "saw" 0.01 0.1 0.7 0.3 # lpf 1200 0.8

# Channel 3: Atmospheric pad
# Slow attack (500ms), gentle decay (300ms), high sustain (80%), long release (1s)
# = Evolving, ambient texture
~d3: synth "c4 e4 g4" "sine" 0.5 0.3 0.8 1.0

# Explicit master mix with effects
# Individual channel levels + master reverb
~master: ~d1 * 0.6 + ~d2 * 0.4 + ~d3 * 0.2 # reverb 0.5 0.5 0.2

# Notes:
# - d1, d2, d3 would auto-route to master if we didn't define ~master explicitly
# - Each channel has its own ADSR character
# - Individual effect on d2 (lpf)
# - Global effect on master (reverb)
# - Reverb params: room_size=0.5, damping=0.5, mix=0.2 (20% wet)
