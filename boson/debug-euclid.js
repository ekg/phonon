#!/usr/bin/env node

const { mini } = require('@strudel/mini');

// Test bd(1,8) - should give 1 hit in 8 steps
const pattern = mini('bd(1,8)');

// Query first cycle
console.log('Query cycle 0-1:');
const events = pattern.queryArc(0, 1);
console.log(`Found ${events.length} events`);

events.forEach(e => {
    console.log(`  Event at ${e.whole.begin} - ${e.whole.end}: ${e.value}`);
});

// Query second cycle
console.log('\nQuery cycle 1-2:');
const events2 = pattern.queryArc(1, 2);
console.log(`Found ${events2.length} events`);

events2.forEach(e => {
    console.log(`  Event at ${e.whole.begin} - ${e.whole.end}: ${e.value}`);
});