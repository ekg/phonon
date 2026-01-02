tempo: 0.5
~drums $ s "bd(3,4) [cp, ~ ~synth]"
~synth $ sine 330
~vowel $ s "~synth(3,17,1)"
out $ ~drums + (0.2 ~/ ~drums # delay 0.5 # lpf 5 # gain 0.1) + ~vowel

