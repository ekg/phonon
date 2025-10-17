# Fixed drum beat with proper levels

# Kick drum - strong but not clipping
~kick = impulse 4 # mul 0.8 # lpf 80 0.95

# Snare/clap with delay
~snare = impulse 2 # delay 0.25 0.0 # mul 0.3 # noise # mul 0.2 # hpf 1500 0.8

# Hihat pattern
~hihat = impulse 8 # mul 0.05 # noise # mul 0.1 # hpf 8000 0.9

# Mix everything
~drums = ~kick + ~snare + ~hihat

# Final output with some filtering
out = ~drums # lpf 3000 0.6 # mul 0.5