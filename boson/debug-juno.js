#!/usr/bin/env node

const { mini } = require('@strudel/mini');

// Test juno sample indexing
const pattern = mini('juno:0 juno:1 juno:2 juno:3');

console.log('Testing juno:0 juno:1 juno:2 juno:3');
const events = pattern.queryArc(0, 1);
console.log(`Found ${events.length} events:`);
events.forEach(e => {
    console.log(`  Time: ${e.whole.begin}`);
    console.log(`    Value:`, e.value);
    console.log(`    Type: ${typeof e.value}`);
    if (Array.isArray(e.value)) {
        console.log(`    Array: ["${e.value[0]}", ${e.value[1]}]`);
    }
});