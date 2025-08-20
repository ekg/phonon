#!/usr/bin/env node

const { mini } = require('@strudel/mini');

// Test sample:index notation
const pattern = mini('notes:0 notes:4 notes:7');

console.log('Testing notes:n pattern:');
const events = pattern.queryArc(0, 1);
events.forEach(e => {
    console.log(`  Event value:`, e.value);
    console.log(`    Type: ${typeof e.value}`);
    if (e.value && typeof e.value === 'object') {
        console.log(`    Keys: ${Object.keys(e.value).join(', ')}`);
    }
});