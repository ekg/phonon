bpm: 135
~bass $ sine 400 # adsr 0.01 0.3 0.6 0.2 # gain 0.4
~d1 $ s "bd(3,8)" # gain 0.3
~d2 $ s "casio(3,8)" # n "<0 1 2 3>" # gain 0.3
~d3 $ ~bass
