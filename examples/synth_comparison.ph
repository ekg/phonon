# Synth Comparison: Basic Oscillators vs SuperDirt Synths
# Shows the difference between basic oscillators + envelope and pre-built synths

tempo: 2.0

# ========== Basic Oscillator + Envelope ==========
# Manual synth building with envelope shaping

~kick_manual: sine 60 # env 0.001 0.3 0.0 0.1

~bass_manual: saw 55 # env 0.001 0.2 0.3 0.1 # lpf 800 1.2

~lead_manual: square 440 # env 0.001 0.1 0.0 0.05

# ========== SuperDirt Synths (Pre-built) ==========
# Pre-configured with internal envelopes

~kick_super: superkick 60 0.5 0.3 0.1

~bass_super: supersaw 55 0.5 7

~lead_super: superchip 440 5.0 0.05

# ========== Comparison ==========

# Manual synths give you FULL control over every parameter
# SuperDirt synths are pre-configured for specific sounds

# Use manual when you want:
# - Complete parameter control
# - Custom envelope shapes
# - Experimental sounds

# Use SuperDirt when you want:
# - Quick, professional sounds
# - Complex synthesis (FM, detuned saws, etc.)
# - Authentic drum sounds

# ========== Output (choose one) ==========

# Manual synths:
out: ~kick_manual * 0.8 + ~bass_manual * 0.4 + ~lead_manual * 0.3

# SuperDirt synths:
# out: ~kick_super * 0.8 + ~bass_super * 0.3 + ~lead_super * 0.3

# Mixed:
# out: ~kick_super * 0.8 + ~bass_manual * 0.4 + ~lead_manual * 0.3
