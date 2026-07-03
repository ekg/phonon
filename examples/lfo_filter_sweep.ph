-- Smooth LFO -> filter modulation (T3 / pt-F5 showcase)
--
-- Phonon's headline feature: patterns ARE control signals, evaluated at SAMPLE
-- RATE. A low-frequency oscillator wired to a filter cutoff must sweep smoothly,
-- with no per-buffer "stairstep" (the ~86 Hz zipper noise that continuous signal
-- patterns used to produce when frozen to their buffer-start value).
--
-- Render:  phonon render examples/lfo_filter_sweep.ph out.wav --duration 8
-- Listen for a clean, gliding filter sweep — no buzzy zipper on the cutoff.

tempo: 0.5

-- 1) Classic slow sine LFO sweeping a lowpass cutoff on a saw drone.
--    cutoff glides continuously between 300 Hz and 2300 Hz at 0.25 Hz.
~lfo1: sine 0.25
~drone1: saw 55
~swept1: ~drone1 # lpf (~lfo1 * 1000 + 1300) 0.7
~voice1: ~swept1 * 0.3

-- 2) Tempo-synced phasor ramp (a continuous 0->1 signal) driving the cutoff.
--    A rising saw-shaped filter sweep, one ramp per cycle.
~ramp2: phasor
~drone2: saw 82.5
~swept2: ~drone2 # lpf (~ramp2 * 2600 + 400) 0.8
~voice2: ~swept2 * 0.22

-- 3) Faster sine LFO modulating resonance for a vocal, wah-like motion.
~lfo3: sine 1.5
~drone3: saw 110
~swept3: ~drone3 # lpf 900 (~lfo3 * 3.0 + 4.0)
~voice3: ~swept3 * 0.2

-- Mix the three smoothly-modulated voices.
out: ~voice1 + ~voice2 + ~voice3
