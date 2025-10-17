# Tidal Cycles Pattern Features Demo
# This file demonstrates all supported Tidal mini-notation features

tempo 2.0  # 120 BPM

# BASIC SEQUENCES
# Uncomment one at a time to hear each pattern type

# Simple sequence
# out: s "bd sn cp hh"

# SUBDIVISION (Repeat)
# out: s "bd*4"              # Four kicks per cycle
# out: s "hh*8"              # Eight hi-hats per cycle
# out: s "bd*2 sn*2"         # Two kicks, then two snares

# RESTS
# out: s "bd ~ sn ~"         # Kick, rest, snare, rest
# out: s "bd ~ ~ ~"          # Kick on beat 1 only
# out: s "~ sn ~ sn"         # Snare on beats 2 and 4 (backbeat!)

# EUCLIDEAN RHYTHMS
# out: s "bd(3,8)"           # 3 kicks in 8 steps (Tresillo)
# out: s "bd(5,16)"          # 5 kicks in 16 steps
# out: s "hh(7,16)"          # 7 hats in 16 steps

# ALTERNATION (Choose)
# out: s "bd <sn cp hh>"     # bd with rotating second sound
# out: s "<bd sn hh>"        # Cycles through: bd, sn, hh
# out: s "bd:0 <sn:0 sn:1 sn:2>"  # Kick with alternating snares

# SAMPLE SELECTION
# out: s "bd:0 bd:1 bd:2 bd:3"    # Different kick samples
# out: s "sn:0 ~ sn:1 ~"          # Alternating snare samples

# LAYERING (Polyrhythms)
# out: s "[bd, hh*8]"             # Kick AND hi-hats together
# out: s "[bd*4, sn*2, hh*8]"     # Three layers
# out: s "[bd(3,8), hh*16, ~ sn ~ sn]"  # Complex layering

# COMBINED FEATURES - Classic House Beat
out = s "[bd*4, hh*8, ~ sn ~ sn]" * 0.8

# Try uncommenting different patterns above to explore!
# Save the file after each change to hear it instantly.
