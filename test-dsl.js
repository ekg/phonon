#!/usr/bin/env node

/**
 * Test the DSL parser
 */

const PatternParser = require('./boson/parser');

const parser = new PatternParser();

// Test patterns
const patterns = [
    '"bd ~ sd ~"',
    '"bd*2 sd hh*4"',
    '"c4 e4 g4 c5"',
    '"[c4,e4,g4] ~"',
    '"bd:0.5 sd:0.2"',
    '"440 550 660"',
    '"kick snare hat ~"'
];

console.log('Testing Pattern DSL Parser\n');

patterns.forEach(pattern => {
    console.log(`Pattern: ${pattern}`);
    const events = parser.parse(pattern);
    const expanded = parser.expand(events);
    
    console.log('Events:', events.map(e => {
        if (e.type === 'rest') return '~';
        if (e.type === 'sample') return `${e.name}(sample)`;
        if (e.type === 'note') return `${e.name}(${Math.round(e.value)}Hz)`;
        if (e.type === 'freq') return `${e.value}Hz`;
        if (e.type === 'chord') return `[${e.notes.map(n => n.name || n.value).join(',')}]`;
        return '?';
    }).join(' '));
    
    console.log('Expanded:', expanded.length, 'events');
    console.log('---');
});