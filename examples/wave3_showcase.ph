-- Wave-3 showcase (wave3-doc-status-refresh)
--
-- One patch that exercises the now-complete melodic + resonant + dynamics
-- surface, so newcomers can hear what actually works together. Everything here
-- is wired into the DSL and render-guarded by tests/test_wave3_showcase.rs
-- (Level-3 audio characteristics: finite, RMS > 0.01, peak <= 1.0, no NaN / DC).
--
--   * scale quantization      n "..." # scale "minor"        (feat-scale-quantization)
--   * chords in mini-notation note "...'maj"  (poly voices)  (feat-chord-support)
--   * resonant filters        # rlpf / # resonz  (RBJ biquad)(feat-resonant-filters)
--   * gate (pattern gate)     gate "t ~ t ~"  -> 0/1 control (wave-3 dynamics)
--   * expander (upward)       # expander thr ratio atk rel   (wave-3 dynamics)
--   * T3-smooth LFO           continuous sine on a cutoff, evaluated per-sample
--                             so the sweep glides with no ~86 Hz zipper stairstep
--
-- Render:  phonon render examples/wave3_showcase.ph out.wav --duration 8
-- Listen for: a scale-locked, rhythmically GATED resonant bass under a warm
-- chord pad lifted by an expander, and a vowel-like resonz lead sweeping on top.

tempo: 0.5

-- 1) SCALE-QUANTIZED RESONANT BASS, rhythmically GATED.
--    A minor-scale degree line drives a saw through a resonant lowpass whose
--    cutoff a slow continuous sine LFO sweeps (T3-smooth). A pattern `gate`
--    chops the sustained tone into a rhythmic 8-step groove.
~degrees: n "0 3 5 7 5 3 2 0" # scale "minor"
~bassfreq: mtof (~degrees + 33)
~lfo: sine 0.25
~bass: saw ~bassfreq # rlpf (~lfo * 900 + 1100) 7.0
~groove: gate "t t ~ t t ~ t ~"
~voice_bass: ~bass * ~groove * 0.28

-- 2) CHORD PAD through a resonant lowpass, shaped by an upward EXPANDER.
--    A i-VI-VII-i minor progression voiced polyphonically through a sine
--    "synth" (base C4), softened by a resonant LPF, then run through an
--    expander that lifts the sustained body above the threshold.
~synth: sine 261.63
~chords: s "~synth*4" # note "a3'min c4'maj d4'min g3'maj"
~pad: ~chords # rlpf 1600 2.0 # expander -34 2.0 0.01 0.15
~voice_pad: ~pad * 0.2

-- 3) FORMANT LEAD — a note-name line through a resonz bandpass swept by a
--    second slow LFO, picking out a moving vowel-like formant.
~lfo2: sine 0.17
~leadsrc: saw (n "a4 c5 e5 d5")
~lead: ~leadsrc # resonz (~lfo2 * 700 + 1200) 10.0
~voice_lead: ~lead * 0.14

-- Mix the three voices.
out: ~voice_bass + ~voice_pad + ~voice_lead
