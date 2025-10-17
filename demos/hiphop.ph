// 🎤 HIP HOP - 90 BPM
// Classic boom bap beat

// Simple version - 4 bar loop with variation
"bd ~ ~ bd ~ ~ bd ~ bd ~ ~ bd ~ ~ ~ ~, ~ ~ sn ~ ~ ~ ~ sn ~ ~ sn ~ ~ ~ sn ~, hh hh hh oh hh hh hh hh hh oh hh hh hh hh oh hh"

// Full Strudel version:
/*
stack(
  // Boom bap kick - varies every 2 bars
  "<[bd ~ ~ bd ~ ~ bd ~] [bd ~ ~ bd ~ bd bd ~]>",
  
  // Snare with ghost notes building over 4 bars
  "<[~ ~ sn ~ ~ ~ ~ sn] [~ ~ sn ~ ~ sn:0.5 ~ sn] [~ sn:0.3 sn ~ ~ ~ sn:0.3 sn] [~ ~ sn ~ ~ ~ ~ sn]>",
  
  // Hi-hat with swing - changes every bar
  "<[hh ~ hh hh ~ hh hh ~] [hh hh ~ hh hh ~ hh hh] [hh ~ hh ~ hh hh ~ hh] [hh*8]>",
  
  // Open hat accent pattern
  "~ ~ ~ oh ~ ~ ~ ~ ~ ~ ~ ~ ~ oh ~ ~".slow(2),
  
  // Walking bassline - 8 bar loop
  "<c2 ~ ~ c2 ~ ~ g1 ~ a1 ~ ~ a1 ~ ~ f1 ~ g1 ~ ~ g1 ~ ~ c2 ~ c2 ~ ~ ~ ~ ~ bb1 ~>".slow(2),
  
  // Vinyl crackle (continuous)
  "vinyl*8",
  
  // Scratch samples - sparse, every 4 bars
  "~ ~ ~ ~ ~ ~ ~ ~ ~ ~ scratch ~ ~ ~ ~ ~".slow(4),
  
  // Jazz sample chop - comes in every 8 bars
  "<~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ jazz:0 ~ jazz:1 ~ ~ jazz:2 ~ ~ ~ ~ ~ ~ ~ ~ ~ ~>".slow(2)
).slow(2)  // Entire pattern over 2 cycles = 8 beats
*/

// Hip hop variations:
// "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8"         // Trap-influenced
// "bd ~ ~ bd ~ bd bd ~, ~ sn ~ ~ ~ sn ~ ~"      // J Dilla style
// "bd(3,8), ~ sn ~ sn, hh*4, ~ ~ oh ~"          // Neo-soul