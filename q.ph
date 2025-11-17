bpm: 120
~x: saw 440 # lpf ("800 <300 500 200 999>" $ fast "<3 19 3 3 2 8 9>")
o1: s "~x(7,17)" $ stut 3 0.25 0.1 # note "c4'min7 f3'min7" # delay 0.334 0.3 # reverb 0.9 0.99
-- if we disable this o1 becomes quieter?? also synthesis problems
o2: s "bd(4,17)"
o3: s "808lt(4,17,2)"