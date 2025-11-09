tempo: 0.2
o1: s "bd(<4 4 4 3>,19)" # gain 0.3
o3: s "hh(<11 28 29 7>,38,<0 1 2>)" # gain (2 * ("0.3 0.4 0.1" $ fast 9)) # reverb 0.9999 0.9
o2: s "cp(2,19,1)" # gain 0.5 # distort 10
o4: s "bassdm(9,19)" # speed ("<0.25 0.33>" $ fast 9)