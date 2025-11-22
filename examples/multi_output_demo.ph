-- Multi-output bus demonstration
-- Shows how o1:, o2:, etc. work in AudioNode mode

tempo: 2.0

-- Kick drum on o1
o1: s "bd"

-- Snare on o2
o2: s "sn"

-- Hi-hat on o3
o3: s "hh*4"

-- All three outputs are automatically mixed together!
-- No need for explicit out: statement
