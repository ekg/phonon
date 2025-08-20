#!/usr/bin/env node

const { mini, note } = require('@strudel/mini');
const { freq, midi } = require('@strudel/core');

// Test if we can convert notes to frequencies
const pattern = mini('c3 e3 g3');

// Try using .note() to convert
const freqPattern = pattern.note().freq();

console.log('Testing note to frequency conversion:');
const events = freqPattern.queryArc(0, 1);
events.forEach(e => {
    console.log(`  Event: ${e.value} Hz`);
});