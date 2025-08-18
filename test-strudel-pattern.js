#!/usr/bin/env node

/**
 * Test Strudel pattern parsing and execution
 * Shows what events would be generated from the pattern
 */

// This demonstrates how Strudel patterns work
// In a real setup, this would need @strudel/core and @strudel/mini

console.log('🎵 Strudel Pattern Analysis');
console.log('===========================\n');

// The pattern from patterns.phonon
const pattern = `
stack(
  "bd*4",                                    // Kick every quarter note
  "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ [~ cp]",   // Claps with variation
  "hh(7,16,2)",                              // Euclidean hi-hats
  "~ ~ oh ~ ~ ~ oh ~ ~ ~ ~ ~ oh ~ ~ ~",     // Open hats
  "<c2 c2 eb2 g2>",                          // Bass progression
  "<~ ~ ~ [c4 eb4] ~ [g4 bb4] ~ c5>*2",      // Lead melody
  "rs(3,8,1)",                               // Euclidean rimshot
  "~ ~ ~ ~ ~ ~ cb ~"                         // Cowbell accent
).slow(2)
`;

console.log('Pattern Code:');
console.log(pattern);
console.log('\n📊 Pattern Breakdown:');
console.log('---------------------\n');

// Simulate one cycle of events (16 steps)
const cycle = [];
for (let i = 0; i < 16; i++) {
    cycle[i] = [];
}

// Layer 1: Kick drum (bd*4) - plays on beats 0, 4, 8, 12
[0, 4, 8, 12].forEach(i => cycle[i].push('bd'));

// Layer 2: Claps - specific pattern
cycle[4].push('cp');
cycle[12].push('cp');
cycle[15].push('cp'); // ghost note

// Layer 3: Hi-hats - euclidean (7,16,2) creates: x..x.x..x.x..x.x
const hhPattern = [1,0,0,1,0,1,0,0,1,0,1,0,0,1,0,1];
hhPattern.forEach((hit, i) => {
    if (hit) cycle[i].push('hh');
});

// Layer 4: Open hats
[2, 6, 12].forEach(i => cycle[i].push('oh'));

// Layer 5: Bass notes (plays every 4 steps, cycling through notes)
const bassNotes = ['c2', 'c2', 'eb2', 'g2'];
[0, 4, 8, 12].forEach((i, idx) => {
    cycle[i].push(bassNotes[idx % 4]);
});

// Layer 6: Lead melody (simplified)
cycle[6].push('c4', 'eb4');
cycle[10].push('g4', 'bb4');
cycle[14].push('c5');

// Layer 7: Rimshot euclidean(3,8,1)
[1, 3, 6].forEach(i => cycle[i].push('rs'));

// Layer 8: Cowbell
cycle[6].push('cb');

// Display the cycle
console.log('Step-by-step breakdown (16th notes):');
console.log('');

for (let i = 0; i < 16; i++) {
    const step = String(i + 1).padStart(2, '0');
    const events = cycle[i].length > 0 ? cycle[i].join(', ') : '~';
    console.log(`Step ${step}: ${events}`);
}

console.log('\n🎼 Pattern Visualization:');
console.log('-------------------------\n');

// Visual grid
const tracks = {
    'Kick  ': [1,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0],
    'Clap  ': [0,0,0,0,1,0,0,0,0,0,0,0,1,0,0,1],
    'HiHat ': hhPattern,
    'Open  ': [0,0,1,0,0,0,1,0,0,0,0,0,1,0,0,0],
    'Bass  ': [1,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0],
    'Lead  ': [0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,0],
    'Rim   ': [0,1,0,1,0,0,1,0,0,0,0,0,0,0,0,0],
    'Cowbell': [0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0]
};

console.log('        1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6');
console.log('        ─────────────────────────────────');

for (const [name, pattern] of Object.entries(tracks)) {
    const visual = pattern.map(x => x ? '●' : '·').join(' ');
    console.log(`${name} ${visual}`);
}

console.log('\n📝 Musical Analysis:');
console.log('-------------------');
console.log('• Tempo: 120 BPM (with .slow(2) = effectively 60 BPM)');
console.log('• Time Signature: 4/4');
console.log('• Key: C minor (C, Eb, G)');
console.log('• Style: Techno/House hybrid with acid influences');
console.log('• Euclidean rhythms create polyrhythmic interest');
console.log('• Layered percussion provides rhythmic complexity');

console.log('\n🎵 To hear this pattern:');
console.log('1. Run: ./phonon start');
console.log('2. The pattern will play through Fermion synth');
console.log('3. Edit patterns.phonon to change it live!');