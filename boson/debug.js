const { pure, early, late, rev, cat, iter, every } = require('./pattern');

// Debug early
console.log('Testing early(0.25, pure("bd"))');
const p1 = early(0.25, pure("bd"));
const events1 = p1.queryArc(0, 1);
console.log('Events:', events1.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));

// Debug late
console.log('\nTesting late(0.25, pure("bd"))');
const p2 = late(0.25, pure("bd"));
const events2 = p2.queryArc(0, 1);
console.log('Events:', events2.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));

// Debug rev
console.log('\nTesting rev(cat(pure("a"), pure("b"), pure("c")))');
const p3 = rev(cat(pure("a"), pure("b"), pure("c")));
const events3 = p3.queryArc(0, 1);
console.log('Events:', events3.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));

// Debug iter
console.log('\nTesting iter(0.25, cat(pure("a"), pure("b"), pure("c"), pure("d")))');
const p4 = iter(0.25, cat(pure("a"), pure("b"), pure("c"), pure("d")));
const events4 = p4.queryArc(0, 1);
console.log('Events:', events4.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));

// Debug every
console.log('\nTesting every(2, rev, cat(pure("a"), pure("b")))');
const p5 = every(2, (pat) => rev(pat), cat(pure("a"), pure("b")));
const events5 = p5.queryArc(0, 1);
console.log('Cycle 0 Events:', events5.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));