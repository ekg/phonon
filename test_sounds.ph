# Test Sounds - Various sounds that should work with phonon render
# Each example is complete on its own - uncomment one "out" line at a time

# 1. Simple sine wave at 440 Hz (A note)
out sine 440 * 0.2

# 2. Pattern of frequencies creating a simple melody
# out sine "220 330 440 330" * 0.2

# 3. Sawtooth bass with filter sweep
# out saw 55 # lpf("200 500 1000 2000", 3) * 0.3

# 4. Square wave lead with fixed filter
# out square "440 550 660 880" # lpf 2000 2 * 0.15

# 5. Filtered noise for percussion
# out noise # lpf("100 100 100 5000", 10) * 0.2

# 6. Low frequency sine for sub bass
# out sine 55 * 0.3

# 7. High-passed noise for hi-hats
# out noise # hpf("8000 10000 8000 6000", 5) * 0.1

# 8. Wobble bass effect
# out saw 110 # lpf("200 2000", 8) * 0.25

# Render with: phonon render test_sounds.phonon output.wav --duration 4