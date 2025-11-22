-- Multi-output bus demonstration with synthesis
-- Shows how o1:, o2:, o3: are automatically mixed

tempo: 2.0

-- Bass on o1
o1: sine 55

-- Mid on o2
o2: sine 110

-- High on o3
o3: sine 220

-- All three outputs are automatically mixed together!
