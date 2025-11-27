# Example 4: LFO-Modulated Filter
# Use a slow sine wave to modulate filter cutoff

tempo: 0.5

# LFO oscillating between 0 and 1
~lfo: sine 0.5 * 0.5 + 0.5

# Bass through filter controlled by LFO
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8

out: ~bass * 0.4
