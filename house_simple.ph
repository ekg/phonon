# 4-on-the-floor House Beat
# Using explicit patterns (euclidean notation doesn't work in DSL yet!)

# Kick - 4 on the floor (every 4th step of 16)
kick = sine 55 * "1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0"

# Clap - on 2 and 4
clap = sine 150 * "0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0"

# Hats - 16th notes with accents
hats = square 8000 * "0.8 0.4 0.8 0.4 0.8 0.4 0.8 0.4 0.8 0.4 0.8 0.4 0.8 0.4 0.8 0.4"

# Bass - simple groove
bass = saw "110 110 165 110"

# Mix
out kick * 0.3 + clap * 0.2 + hats * 0.05 + bass * 0.2
