-- Synths and Effects Demo
-- Showcasing all 7 SuperDirt synths and 4 effects

tempo 2.0  # 120 BPM

-- SUPERKICK - Analog kick drum
-- Parameters: freq, pitch_env, sustain, noise
~kick_synth = superkick(60, 0.5, 0.15, 0.2) * 0.6

-- SUPERSAW - Detuned sawtooth
-- Parameters: freq, amp, detune
-- Pattern frequency creates a bassline
~saw_bass = supersaw("55 82.5 110 82.5", 0.4, 5)

-- SUPERPWM - Pulse-width modulation
-- Parameters: freq, amp, pwm_rate
~pwm_lead = superpwm(440, 0.3, 2.5)

-- SUPERCHIP - Chiptune square wave
-- Parameters: freq, slide, decay
~chip_arp = superchip("220 330 440 330", 2.0, 0.12) * 0.3

-- SUPERFM - FM synthesis
-- Parameters: freq, mod_freq, mod_index
~fm_bell = superfm(440, 2.0, 1.5) * 0.2

-- SUPERSNARE - Snare drum synth
-- Parameters: freq, tone, decay
~snare_synth = supersnare(200, 0.8, 0.15) * 0.5

-- SUPERHAT - Hi-hat synth
-- Parameters: amp, decay
~hat_synth = superhat(0.6, 0.08)

-- SAMPLES for rhythm
~kick = s("bd*4", "1.0 0.8 0.95 0.85")
~hats = s("hh*8", 0.6)
~snare = s("~ sn ~ sn", 0.9)

-- EFFECT 1: REVERB
-- Parameters: room_size, damping, wet
~verb = reverb(~kick + ~hats + ~snare, 0.6, 0.5, 0.3)

-- EFFECT 2: DISTORTION
-- Parameters: drive, mix
~distorted_bass = dist(~saw_bass, 3.5, 0.6)

-- EFFECT 3: BITCRUSH
-- Parameters: bits, rate
~crushed_chip = bitcrush(~chip_arp, 6.0, 4.0)

-- EFFECT 4: CHORUS
-- Parameters: rate, depth, mix
~chorus_pad = chorus(~pwm_lead, 0.8, 0.4, 0.5)

-- FINAL MIX
out = (
    ~verb +                    # Drums with reverb
    ~distorted_bass * 0.3 +   # Distorted bass
    ~chorus_pad * 0.2 +       # PWM pad with chorus
    ~crushed_chip * 0.15 +    # Bitcrushed chiptune
    ~fm_bell * 0.15           # FM bell
) * 0.7

-- EXPERIMENTATION TIPS:
--
-- Try different synth parameters:
-- - superkick: vary pitch_env (0.3-0.7) for different kick tones
-- - supersaw: increase detune (10-20) for wider sound
-- - superpwm: change pwm_rate (0.5-5.0) for different textures
-- - superchip: adjust slide (0-5) for pitch bends
-- - superfm: vary mod_index (0.5-3.0) for harmonic content
--
-- Effect chains:
-- - Try: reverb(chorus(input)) for lush textures
-- - Try: dist(bitcrush(input)) for harsh sounds
-- - Try: chorus(reverb(input)) for spacey pads
--
-- Pattern parameters work on synths too!
-- - superkick(60, "0.3 0.7 0.5", 0.1, 0.2)
-- - supersaw("55 82.5 110", 0.4, 5)
