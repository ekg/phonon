-- Standard 120 BPM drum pattern

-- 120 BPM = 2 beats per second
-- 1 bar = 4 beats = 2 seconds

-- Kick drum: quarter notes (on 1, 2, 3, 4) = 2 Hz
~kick = impulse 2 # mul 0.6 # lpf 70 0.95

-- Snare: on beats 2 and 4 = 1 Hz, delayed by 0.5s
~snare = impulse 1 # delay 0.5 0.0 # mul 0.4 # hpf 1000 0.7

-- Hi-hat: 8th notes = 4 Hz  
~hihat = impulse 4 # mul 0.1 # hpf 8000 0.9

-- Classic house pattern
out = ~kick + ~snare + ~hihat # mul 0.7