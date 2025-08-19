const { euclid, euclidRot, euclidLegato } = require('./pattern');

// Test euclid(3, 8)
console.log('euclid(3, 8):');
const p1 = euclid(3, 8);
const events1 = p1.queryArc(0, 1);
console.log('Events:', events1.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat()
})));

// Test euclidRot(3, 8, 1)
console.log('\neuclidRot(3, 8, 1):');
const p2 = euclidRot(3, 8, 1);
const events2 = p2.queryArc(0, 1);
console.log('Events:', events2.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat()
})));

// Test euclidLegato(3, 8)
console.log('\neuclidLegato(3, 8):');
const p3 = euclidLegato(3, 8);
const events3 = p3.queryArc(0, 1);
console.log('Events:', events3.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    duration: e.part.end.toFloat() - e.part.begin.toFloat()
})));