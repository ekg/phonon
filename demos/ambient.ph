// 🌊 AMBIENT - 70 BPM
// Ethereal soundscape

// Simple version - very slow 32 beat cycle
"bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ hh ~ ~ ~ ~ ~ ~"

// Full Strudel version:
/*
stack(
  // Very sparse kick
  "bd/4",
  
  // Occasional soft snare
  "~ ~ ~ ~ ~ ~ ~ ~ sn/8",
  
  // Gentle hi-hat whispers
  "~ ~ ~ hh/16",
  
  // Slow evolving bassline
  "<c2 ~ ~ ~ ~ ~ g2 ~ ~ ~ ~ ~ eb2 ~ ~ ~ ~ ~ bb1 ~ ~ ~ ~ ~>/4",
  
  // Ambient pad chords (long sustained)
  "[c3,e3,g3,c4]/8 ~ ~ ~ [a2,c3,e3,a3]/8 ~ ~ ~ [f2,a2,c3,f3]/8 ~ ~ ~",
  
  // Melodic fragments
  "~ ~ ~ c5/2 ~ ~ ~ ~ e5/2 ~ ~ ~ ~ g5/2 ~ ~ ~ ~ c6/4 ~ ~ ~ ~",
  
  // Rain/nature sounds
  "rain*2",
  
  // Reverse cymbal swells
  "~ ~ ~ ~ ~ ~ ~ ~ revcrash/4"
).slow(4)  // Make everything even slower
*/

// Ambient variations:
// "bd/8, <c2 e2 g2 c3>/4"                       // Minimal drone
// "~ ~ ~ bd/4, [c3,g3,c4]/16"                   // Beatless
// "field*1"                                      // Field recording only