# Synth and Effects Demo
# Demonstrates the SuperDirt-inspired synth library and audio effects

cps: 2.0

# Example 1: SuperSaw with reverb
# out: reverb (supersaw 110 0.5 7) 0.8 0.5 0.3

# Example 2: Kick drum with distortion and reverb
# out: reverb (dist (superkick 60 0.5 0.3 0.1) 3.0 0.3) 0.7 0.5 0.4

# Example 3: PWM synth with chorus
# out: chorus (superpwm 220 0.5 0.8) 1.5 0.6 0.4

# Example 4: FM bells with reverb
# out: reverb (superfm 440 2.0 1.5) 0.9 0.3 0.5

# Example 5: Bitcrushed chip sound
# out: bitcrush (superchip 880 6.0 0.05) 4.0 8.0 * 0.5

# Example 6: Full drum kit with effects
# ~kick: superkick 60 0.5 0.3 0.1
# ~snare: supersnare 200 0.8 0.15
# ~hat: superhat 0.7 0.05
# out: reverb (~kick + ~snare + ~hat) 0.6 0.5 0.2 * 0.3

# Example 7: Effects chain
out: reverb (chorus (dist (supersaw 110 0.5 5) 3.0 0.3) 1.0 0.5 0.3) 0.7 0.5 0.4 * 0.2
