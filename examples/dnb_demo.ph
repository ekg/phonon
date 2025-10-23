# Drum & Bass Demo
# Fast breakbeats and heavy bass

# 180 BPM (fast!)
tempo: 3.0

# KICK PATTERN
~kick: s "bd*2 bd ~ bd ~ bd ~"

# SNARE PATTERN
~snare: s "~ sn ~ sn"

# HI-HATS
~hats: s "hh*32" * 0.4

# Open hi-hat accents
~open: s "~ ~ ~ ~ ~ ~ oh ~" * 0.5

# BREAKS
~perc: s "cp(7,16)" * 0.3

# Combine drums
~drums: ~kick + ~snare + ~hats + ~open + ~perc

# BASS (Reese bass style)
~bass: supersaw "55 55 55 82.5" 0.6 12

# Filter sweep for bass
~lfo: sine 0.5 * 0.5 + 0.5
~bass_filtered: ~bass # lpf (~lfo * 1500 + 400) 0.85

# Distort the bass
~bass_dist: distort ~bass_filtered 4.0 0.7

# SUB BASS
~sub: sine "55 55 55 82.5" * 0.3

# PAD
~pad: superfm 220 1.5 0.8 * 0.06

# EFFECTS
~drums_verb: reverb ~drums 0.3 0.5 0.15
~pad_chorus: chorus ~pad 0.6 0.3 0.5

# FINAL MIX
out: ~drums_verb + ~bass_dist * 0.3 + ~sub * 0.2 + ~pad_chorus * 0.15 * 0.7

# D&B PRODUCTION TIPS:
# 1. Keep kick punchy
# 2. Use fast hi-hats (32nd notes) for energy
# 3. Heavy bass with distortion and detuning
# 4. Sub bass provides low-end foundation
# 5. Tempo: 170-180 BPM
