--o1 $ s "blip(1,2)" # n 0 # note ("-24'maj7" + "<0 3 4 7 0 3 4 12>") # adsr 0.01 1 1 1

bpm: 120
o1 $ stack [s "bd(11,17,2)" # note ("<c6 e2 d3 g4>" $ fast 9) # gain 2 # delay 0.3 0.2,
(s "bd(3,17,2)" ~* s "bd(9,17,8)" ~* (sine 220)) # gain 1]
# reverb :room_size 2.0 :damping 0.2 :mix 0.9 # lpf 2000
o2 $ s "alphabet:0(1,17,3)" # speed "-0.5" # reverb :room_size 0.9 :damping 0.2 :mix 0.9 
o3 $ s "alphabet:2(1,17,9)" # speed "-0.5" # delay 1.125 0.8 # reverb :room_size 0.9 :damping 0.2 :mix 0.2 