const { off, pure } = require('./pattern');

const p = off(0.25, pat => pure("sn"), pure("bd"));
const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());

console.log('off(0.25, pat => pure("sn"), pure("bd"))');
console.log('Events:', events.map(e => ({
    start: e.part.begin.toFloat(),
    end: e.part.end.toFloat(),
    value: e.value
})));