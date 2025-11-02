-- Sample Bank Selection Demo
-- Demonstrates the `n` parameter and colon syntax for sample variation

tempo: 2.0

-- METHOD 1: Using n parameter with patterns
-- This cycles through samples dynamically
~kick1: s "bd*4" # n "0 1 2 3"

-- METHOD 2: Direct colon syntax in mini-notation
-- Explicit sample selection
~kick2: s "bd:0 bd:1 bd:2 bd:3"

-- METHOD 3: Combining both approaches
-- Pattern select which samples to cycle through
~snare: s "sn*4" # n "<0 1 2>"

-- METHOD 4: Random-ish variation with large n values
-- These wrap around based on sample bank size
-- If bd has 3 samples: n=5 → 5%3=2, n=7 → 7%3=1, etc.
~random_kick: s "bd*8" # n "0 5 2 7 1 6 3 4"

-- METHOD 5: Colon syntax with euclidean rhythms
~hats: s "hh:0(3,8) hh:1(5,8,2)"

-- METHOD 6: Alternating samples with colon syntax
~perc: s "<cp:0 cp:1 cp:2>"

-- Mix it all together
out: (~kick1 + ~snare + ~hats) * 0.25
