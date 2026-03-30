~cutoffs: "<300 200 1000>" $ fast 7
~resonances: "<0.8 0.6 0.2>" $ fast 13
out:  s "hh*4 cp" # lpf ~cutoffs ~resonances # gain 2  # compressor 0.9 0.1 0.01 0.2 1
--o1: s "bd(3,8)" # lpf ("2 0.5 1.5" * ~cutoffs) 0.5 # compressor 0.9 0.1 0.01 0.2 1
--o2: s "cp(2,4,2)"
o3: sine # note "e3'maj7(3,8,1)" # adsr 0.01 0.9 0.9 1 # gain 0.5
o4: s "cp" # note "e4"