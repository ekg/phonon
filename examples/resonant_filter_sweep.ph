-- Resonant filter sweep showcase (RLPF / RHPF / Resonz)
--
-- The resonant filters a live coder reaches for: a plain LPF only rolls off
-- highs, but RLPF / RHPF / Resonz add a tunable resonance PEAK at the cutoff.
-- Because patterns ARE control signals in Phonon (evaluated at sample rate),
-- an LFO wired to the cutoff produces the classic gliding "acid" sweep with a
-- singing resonant peak riding on top — no per-buffer zipper stairstep.
--
-- Render:  phonon render examples/resonant_filter_sweep.ph out.wav --duration 8
-- Listen for: a squelchy resonant sweep (voice 1), an airy resonant top-end
-- shimmer (voice 2), and a vowel-like formant tone (voice 3).

tempo: 0.5

-- 1) ACID BASSLINE — saw through a resonant lowpass (RLPF).
--    A slow sine LFO sweeps the cutoff 200 Hz -> 2200 Hz while the resonance
--    stays high (Q ~8), so the filter "sings" at the cutoff as it glides.
~lfo1: sine 0.25
~bass1: saw 55
~acid1: ~bass1 # rlpf (~lfo1 * 1000 + 1200) 8.0
~voice1: ~acid1 * 0.3

-- 2) RESONANT AIR — saw through a resonant highpass (RHPF).
--    Sweeping the cutoff upward thins the tone while the resonance peak adds a
--    whistling brightness that tracks the cutoff.
~lfo2: sine 0.18
~src2: saw 110
~air2: ~src2 # rhpf (~lfo2 * 1400 + 1800) 6.0
~voice2: ~air2 * 0.18

-- 3) FORMANT / VOWEL — saw through a resonant bandpass (Resonz).
--    A narrow high-Q bandpass swept across the harmonics of a low saw picks out
--    a moving formant, giving a vowel-like "wah" character.
~lfo3: sine 0.33
~src3: saw 82.5
~vowel3: ~src3 # resonz (~lfo3 * 600 + 900) 12.0
~voice3: ~vowel3 * 0.35

-- Mix the three resonant voices.
out: ~voice1 + ~voice2 + ~voice3
