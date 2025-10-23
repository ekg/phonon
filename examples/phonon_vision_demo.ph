# PHONON VISION DEMO
# ==================
# This demonstrates what makes Phonon unique vs Tidal Cycles:
# PATTERNS CAN MODULATE AUDIO PARAMETERS AT SAMPLE RATE!

# 120 BPM
tempo: 2.0

# ========== THE UNIQUE FEATURE ==========
#
# In Tidal/Strudel, patterns only trigger discrete events.
# In Phonon, patterns ARE control signals that evaluate at 44.1kHz!
#
# This means you can use patterns to modulate ANY audio parameter
# continuously, just like an LFO or envelope.

# Example 1: Pattern-Controlled Filter Cutoff
# ---------------------------------------------
# Create an LFO from a pattern
# Slow sine wave (one cycle every 4 seconds)
~lfo: sine 0.25

# Bass with pattern-modulated filter (IMPOSSIBLE in Tidal!)
~bass: saw 110 # lpf (~lfo * 2000 + 500) 0.8

# The filter cutoff sweeps smoothly from 500Hz to 2500Hz
# controlled by the ~lfo pattern!

# Example 2: Drums with Pattern Modulation
# ------------------------------------------
~drums: s "bd sn hh*4 cp"

# Modulate drums with the same LFO
~drums_mod: ~drums * (~lfo * 0.3 + 0.7)

# Example 3: Pad with Evolving Brightness
# -----------------------------------------
# Modulate pad brightness with LFO
~pad: supersaw 220 0.15 7 # lpf (~lfo * 2000 + 800) 0.6

# ========== FINAL MIX ==========
out: ~bass * 0.4 + ~drums_mod * 0.6 + ~pad * 0.15

# ========== WHY THIS MATTERS ==========
#
# In Tidal Cycles:
#   d1 $ sound "bd sn"  -- Can only trigger events
#   d1 $ lpf 1000 $ sound "bd"  -- Filter is STATIC per event
#
# In Phonon:
#   ~lfo: sine 0.25
#   out: saw 110 # lpf (~lfo * 2000 + 500) 0.8
#   -- Filter sweeps CONTINUOUSLY within events!
#
# This unlocks:
# - True analog-style modulation
# - Cross-modulation between any signals
# - Audio-rate pattern evaluation
# - Evolving, breathing, organic sounds
#
# This is the CORE VISION of Phonon: Everything can modulate everything.
