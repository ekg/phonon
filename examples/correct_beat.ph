-- Correctly structured drum beat

-- Kick - just filtered impulse
~kick = impulse 4 # mul 0.8 # lpf 80 0.95

-- Snare - gated noise (multiply impulse BY noise, not chain after)
~snare_trig = impulse 2 # delay 0.25 0.0
~snare_noise = noise # mul 0.2 # hpf 1500 0.8
~snare = ~snare_trig * ~snare_noise # mul 0.5

-- Hihat - also gated noise
~hihat_trig = impulse 8
~hihat_noise = noise # mul 0.1 # hpf 8000 0.9
~hihat = ~hihat_trig * ~hihat_noise # mul 0.3

-- Mix
out = ~kick + ~snare + ~hihat # mul 0.5