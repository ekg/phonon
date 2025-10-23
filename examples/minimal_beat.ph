-- Minimal techno beat - 120 BPM

-- Kick on every beat (120 BPM = 2 Hz)
~kick = impulse 2 # mul 0.5 # lpf 60 0.98

-- Hihat on 8th notes (4 Hz at 120 BPM)
~hat = impulse 4 # mul 0.15 # hpf 10000 0.95

out = ~kick + ~hat # mul 0.5