-- Simple working beat without signal multiplication

-- Kick drum - just filtered impulse
~kick = impulse 4 # mul 0.6 # lpf 80 0.95

-- Simple hihat - just filtered impulse (no noise)
~hihat = impulse 8 # mul 0.2 # hpf 8000 0.9

-- Simple snare - use pink noise instead of white (less harsh)
~snare = impulse 2 # delay 0.25 0.0 # mul 0.3 # lpf 2000 0.7

-- Mix everything at low volume
out = ~kick + ~hihat + ~snare # mul 0.4