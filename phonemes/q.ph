bpm: 120
~synth $ saw # lpf 800
-- # lpf ("800 <300 500 200 999>" $ fast "<3 19 3 3 2 8 9>") 0.7 # delay 0.334 0.3 # reverb 0.9 0.99
~d1 $ s "808bd(4,17)" # n 3 # gain 1
~d2 $ s "808ht(4,17,<2 3 1 2>)"
~d3 $ s "~synth(3,17,1)" # note "<c'min7 g'min7>"