# Classic House Track
# Four-on-the-floor with bass and pads

tempo: 2.0  # 120 BPM

# DRUMS

# Four-on-the-floor kick
~kick: s "bd*4"

# Eighth-note hi-hats
~hats: s "hh*8" * 0.6

# Snare on 2 and 4 (backbeat)
~snare: s "~ sn ~ sn"

# Open hi-hat accents
~open: s "~ ~ ~ oh" * 0.4

# Percussive elements
~perc: s "~ ~ cp ~" * 0.3

# Combine drums
~drums: ~kick + ~hats + ~snare + ~open + ~perc

# BASS

# Walking bassline with pattern frequency
~bass: supersaw "55 55 82.5 55" 0.4 5

# Filter the bass
~bass_filtered: ~bass # lpf 1200 0.9

# PADS

# Detuned pad for warmth
~pad: supersaw 220 0.15 12 # lpf 2500 0.7

# FM pad for texture
~fm_pad: superfm 330 2.0 0.8 * 0.08

# EFFECTS

# Add reverb to drums
~drums_verb: reverb ~drums 0.5 0.5 0.25

# Chorus on pads
~pads_chorus: chorus (~pad + ~fm_pad) 0.8 0.4 0.5

# FINAL MIX
out: (~drums_verb + ~bass_filtered * 0.25 + ~pads_chorus * 0.25) * 0.7

# LIVE CODING TIP:
# Comment/uncomment parts to build the track live:
# 1. Start with just ~kick
# 2. Add ~hats
# 3. Add ~snare
# 4. Bring in ~bass
# 5. Layer the ~pads
# 6. Full mix!
