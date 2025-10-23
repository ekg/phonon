-- Real drum samples from dirt-samples - 120 BPM

-- Kick drum sample on every beat (2 Hz at 120 BPM)
~kick = impulse 2 * sp bd

-- Clap sample on beats 2, 4 (1 Hz, offset by 0.5s)  
~clap = impulse 1 # delay 0.5 0.0 * sp cp

-- Hi-hat sample on 8th notes (4 Hz)
~hihat = impulse 4 * sp hh

-- Mix the real samples
out = ~kick + ~clap + ~hihat # mul 0.8