--o1 $ s "808bd(3,8)" # n 4 # speed 0.5
~b $ stack [s "bd(4,7)" # speed 0.33, s "808bd(3,7)" # n 4 # speed -0.66 # begin 0.5]
~a $ s "em2(5,7,1)" # n ("<5 8 2 9>" $ fast 9) # ar 0.1 0.2 # gain "<0 1 2>" # speed "<0.2 0.2 0.1 1>"
~c $ s "~synth" # gain 0.5
out $ ~a * 0.7 + ~b
~synth $ saw 440 
--o3 $ s "~synth(3,8,1)" # note "c1 d1" # adsr 0.1 1 1 1  # lpf 100