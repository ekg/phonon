#!/usr/bin/env node

const { mini } = require('@strudel/mini');

// Test note parsing
const patterns = [
    'c3',
    '[c3,e3,g3]',
    '<c3 g3 a3 f3>',
    '<[c3,e3,g3] [g3,b3,d4]>'
];

patterns.forEach(p => {
    console.log(`\nPattern: "${p}"`);
    const pat = mini(p);
    const events = pat.queryArc(0, 1);
    events.forEach(e => {
        console.log(`  Event: ${JSON.stringify(e.value)} at ${e.whole.begin}`);
    });
});