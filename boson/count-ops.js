#!/usr/bin/env node

const p = require('./pattern');
const ops = Object.keys(p).filter(k => 
    typeof p[k] === 'function' && 
    !['Fraction', 'TimeSpan', 'Event', 'Pattern'].includes(k)
);

console.log('Total operators:', ops.length);
console.log('\nOperators:', ops.sort().join(', '));