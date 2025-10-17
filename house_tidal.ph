# House beat using ACTUAL Tidal mini-notation
# Let's see what works!

# Test 1: Euclidean rhythm - "1(4,16)" means 4 pulses in 16 steps
kick = sine 55 * "1(4,16)"

# Test 2: Explicit pattern with rests
snare = sine 150 * "~ 1 ~ 1"

# Test 3: Repetition and subdivision
hats = square 9000 * "[1 0.5]*8"

# Test 4: Bass pattern with actual frequencies
bass = saw "[110 110 ~ 165]*2"

# Stack them all
out kick * 0.3 + snare * 0.2 + hats * 0.05 + bass * 0.2
