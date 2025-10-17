# Live Coding Session
# Edit this file and save to hear changes!
# Try uncommenting different sections:

# === Basic Drum Beat ===
~kick = impulse 4 # mul 0.8 # lpf 80 0.95
~clap = impulse 2 # delay 0.25 0.0 # mul 0.3 # noise # mul 0.2 # hpf 1200 0.7
~hihat = impulse 8 # mul 0.1 # noise # mul 0.1 # hpf 7000 0.9
~drums = ~kick + ~clap + ~hihat
out = ~drums # lpf 2500 0.6 # mul 0.5

# === Faster Beat (uncomment to try) ===
# ~kick: impulse 8 # mul 0.6 # lpf 60 0.95
# ~snare: impulse 4 # delay 0.125 0.0 # mul 0.2 # noise # mul 0.2 # hpf 1500 0.8
# ~hihat: impulse 16 # mul 0.05 # noise # mul 0.1 # hpf 8000 0.95
# ~drums: ~kick + ~snare + ~hihat
# out: ~drums # lpf 4000 0.5 # mul 0.7

# === Minimal Techno (uncomment to try) ===
# ~kick: impulse 4 # mul 0.9 # lpf 50 0.98
# ~bass: impulse 4 # delay 0.125 0.0 # mul 0.3 # lpf 200 0.9
# ~tick: impulse 16 # mul 0.05 # hpf 10000 0.95
# out: ~kick + ~bass + ~tick # mul 0.6

# === Ambient Percussion (uncomment to try) ===
# ~pulse: impulse 2 # mul 0.3 # lpf 300 0.7 # reverb 0.8 0.3
# ~shimmer: impulse 8 # mul 0.05 # noise # mul 0.05 # hpf 12000 0.9 # delay 0.125 0.4
# out: ~pulse + ~shimmer # mul 0.5

# === Experiment! ===
# Try changing:
# - impulse frequencies (2, 4, 8, 16)
# - mul values (volume)
# - lpf/hpf cutoff frequencies
# - delay times (0.125, 0.25, 0.5)
# - Mix different elements