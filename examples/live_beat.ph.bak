# Live coding drum beat example
# Edit this file while running: phonon live examples/live_beat.phonon

# Basic 4-on-floor kick + clap pattern
~kick = impulse 4 # mul 100 # lpf 80 0.95
~clap = impulse 2 # delay 0.25 0.0 # mul 50 # noise # mul 0.4 # hpf 1200 0.7
~hihat = impulse 8 # mul 15 # noise # mul 0.2 # hpf 7000 0.9

# Mix drums
~drums = ~kick + ~clap + ~hihat

# Output with filter
out = ~drums # lpf 2500 0.6 # mul 0.8