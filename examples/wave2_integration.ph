-- Wave-2 integration showcase (verify-feature-wave2)
--
-- Exercises the whole feature wave-2 surface end-to-end in one musical patch:
--   * scale quantization   — `n "..." # scale "minor"`      (feat-scale-quantization)
--   * chords in mini-notation — `note "...'maj"`             (feat-chord-support)
--   * resonant filter      — `# rlpf cutoff Q` (RBJ biquad)  (feat-resonant-filters)
--   * T3-smooth LFO        — a continuous `sine` pattern wired to the filter
--     cutoff, evaluated per-sample so the sweep glides with NO ~86 Hz zipper
--     stairstep                                              (promote-t3-continuous-patterns)
--   * f64 trigger timing keeps onsets sample-accurate        (promote-t2-trigger-f64)
--
-- Render:  phonon render examples/wave2_integration.ph out.wav --duration 8
-- Listen for: a squelchy scale-locked resonant bassline gliding under a warm
-- chord pad and a bright note-name lead — no zipper, no clipping, no blow-up.

tempo: 0.5

-- 1) SCALE-QUANTIZED RESONANT BASS.
--    A minor-scale degree line drives a saw; the saw runs through a resonant
--    lowpass whose cutoff is swept by a slow continuous sine LFO (T3-smooth).
~degrees: n "0 3 5 7 5 3 2 0" # scale "minor"
~bassfreq: mtof (~degrees + 33)
~lfo: sine 0.25
~bass: saw ~bassfreq # rlpf (~lfo * 900 + 1100) 7.0
~voice_bass: ~bass * 0.28

-- 2) CHORD PAD — a i-VI-VII-i minor progression voiced through a sine "synth"
--    (base C4) and softened with a gentle resonant lowpass.
~synth: sine 261.63
~chords: s "~synth*4" # note "a3'min c4'maj d4'min g3'maj"
~pad: ~chords # rlpf 1600 2.0
~voice_pad: ~pad * 0.22

-- 3) NOTE-NAME LEAD — an absolute-pitch melodic line an octave up, thinned by a
--    resonant highpass for air, sweetened by a second slow LFO on its cutoff.
~lfo2: sine 0.17
~leadsrc: sine (n "a4 c5 e5 d5")
~lead: ~leadsrc # rhpf (~lfo2 * 700 + 1200) 4.0
~voice_lead: ~lead * 0.14

-- Mix the three wave-2 voices.
out: ~voice_bass + ~voice_pad + ~voice_lead
