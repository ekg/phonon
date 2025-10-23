-- Envelope Demo
-- Demonstrates ADSR envelope shaping for oscillators

tempo: 2.0

-- ========== Envelope Parameters ==========
-- env(attack, decay, sustain_level, release)
-- - attack: Time to reach peak (seconds)
-- - decay: Time to decay to sustain level (seconds)
-- - sustain_level: Level to hold (0.0-1.0)
-- - release: Time to fade to silence (seconds)

-- ========== Pluck Sound (Guitar/Piano) ==========
-- Fast attack, no sustain, quick release
~pluck: sine 440 # env 0.001 0.3 0.0 0.1

-- ========== Pad Sound (Strings/Atmosphere) ==========
-- Slow attack, high sustain, slow release
~pad: saw 220 # env 0.5 0.3 0.8 0.4

-- ========== Bass Sound (Punchy Bass) ==========
-- Fast attack, medium sustain, quick release
~bass: saw 55 # env 0.001 0.2 0.3 0.1 # lpf 800 1.2

-- ========== Lead Sound (Synth Lead) ==========
-- Very fast attack, no sustain (staccato)
~lead: square 880 # env 0.001 0.1 0.0 0.05

-- ========== Percussion from Noise ==========
-- Hi-hat style: very short envelope on filtered noise
~hh: noise 0 # env 0.001 0.05 0.0 0.02 # hpf 8000 2.0

-- ========== Mix ==========
out: ~pluck * 0.3 + ~pad * 0.2 + ~bass * 0.4 + ~lead * 0.3 + ~hh * 0.3
