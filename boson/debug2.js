const { pure, early, late, rev, cat, iter, every } = require('./pattern');

// Debug rev - sorted by start time
console.log('Testing rev(cat(pure("a"), pure("b"), pure("c")))');
const p3 = rev(cat(pure("a"), pure("b"), pure("c")));
const events3 = p3.queryArc(0, 1);
const sorted3 = events3.sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
console.log('Events (sorted):', sorted3.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));

// Debug iter(0.25) - what should rotate by one position
console.log('\nTesting iter(0.25, cat(pure("a"), pure("b"), pure("c"), pure("d")))');
console.log('Original pattern would be: a(0-0.25), b(0.25-0.5), c(0.5-0.75), d(0.75-1)');
console.log('Rotated by 0.25 should be: d(0-0.25), a(0.25-0.5), b(0.5-0.75), c(0.75-1)');
const p4 = iter(0.25, cat(pure("a"), pure("b"), pure("c"), pure("d")));
const events4 = p4.queryArc(0, 1);
const sorted4 = events4.sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
console.log('Actual events:', sorted4.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));

// Test pure across boundaries
console.log('\nTesting pure("bd") with early');
const pureEvents = pure("bd").queryArc(0.25, 1.25);
console.log('Pure events from 0.25 to 1.25:', pureEvents.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    wholeStart: e.whole ? e.whole.begin.toFloat() : null,
    wholeEnd: e.whole ? e.whole.end.toFloat() : null
})));