tempo: 2.0
-- Comprehensive test of sample bank selection (colon notation)

-- Multiple BD samples cycling
~kicks: s "bd:0 bd:1 bd:2 bd:3"

-- Multiple SN samples with fast transform
~snares: s "sn:0 sn:1" $ fast 2

-- Mix different HH samples
~hats: s "hh:0*4 hh:1*4 hh:2*4"

-- CP samples with alternation
~claps: s "[cp:0 cp:1] [cp:2 cp:3]"

-- Combine all with different levels
out: (~kicks * 0.8 + ~snares * 0.6 + ~hats * 0.4 + ~claps * 0.5) * 0.6
