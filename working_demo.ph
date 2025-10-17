# Working Demo - Compatible with current parser
tempo 2.0

# Simple oscillator with filter
out saw 110 # lpf("100 500 1000 2000", 5) * 0.2

# Other lines to try (one at a time):
# out sine "440 880 660 550" * 0.2
# out saw 55 # lpf 1000 3 * 0.3
# out noise # lpf("100 100 100 5000", 8) * 0.2