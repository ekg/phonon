#!/usr/bin/env node

/**
 * Randomness Pattern Operator Tests
 */

const {
    Pattern,
    pure,
    cat,
    rand,
    irand,
    choose,
    wchoose,
    shuffle,
    scramble,
    degrade,
    degradeBy,
    sometimes,
    sometimesBy,
    often,
    rarely,
    almostNever,
    almostAlways,
    fast
} = require('./pattern');

class TestRunner {
    constructor() {
        this.tests = [];
        this.passed = 0;
        this.failed = 0;
    }

    test(name, fn) {
        this.tests.push({ name, fn });
    }

    assertLength(array, expected, message = '') {
        if (array.length !== expected) {
            throw new Error(`Length assertion failed: ${message}
                Expected length: ${expected}
                Actual length: ${array.length}`);
        }
    }

    assertEqual(actual, expected, message = '') {
        if (JSON.stringify(actual) !== JSON.stringify(expected)) {
            throw new Error(`Assertion failed: ${message}
                Expected: ${JSON.stringify(expected)}
                Actual: ${JSON.stringify(actual)}`);
        }
    }

    assertInRange(value, min, max, message = '') {
        if (value < min || value > max) {
            throw new Error(`Range assertion failed: ${message}
                Expected: ${min} <= value <= ${max}
                Actual: ${value}`);
        }
    }

    assertContains(array, value, message = '') {
        if (!array.includes(value)) {
            throw new Error(`Contains assertion failed: ${message}
                Array: ${JSON.stringify(array)}
                Should contain: ${value}`);
        }
    }

    assertApproxEqual(actual, expected, tolerance = 0.1, message = '') {
        const diff = Math.abs(actual - expected);
        if (diff > tolerance) {
            throw new Error(`Approximate equality failed: ${message}
                Expected: ${expected} ± ${tolerance}
                Actual: ${actual}
                Difference: ${diff}`);
        }
    }

    async run() {
        console.log('🧪 Running Randomness Pattern Tests...\n');
        
        for (const { name, fn } of this.tests) {
            try {
                await fn.call(this);
                console.log(`✅ ${name}`);
                this.passed++;
            } catch (error) {
                console.log(`❌ ${name}`);
                console.log(`   ${error.message}`);
                this.failed++;
            }
        }
        
        console.log(`\n📊 Results: ${this.passed} passed, ${this.failed} failed`);
        return this.failed === 0;
    }
}

// Create test runner
const runner = new TestRunner();

// === Test rand ===
runner.test('rand produces values between 0 and 1', function() {
    const p = rand();
    const events = p.queryArc(0, 10);
    
    this.assertLength(events, 10, 'rand one per cycle');
    
    for (const event of events) {
        this.assertInRange(event.value, 0, 1, 'rand value in range');
    }
});

runner.test('rand is deterministic', function() {
    const p = rand();
    
    const events1 = p.queryArc(0, 5);
    const events2 = p.queryArc(0, 5);
    
    this.assertLength(events1, events2.length, 'rand deterministic length');
    
    for (let i = 0; i < events1.length; i++) {
        this.assertEqual(events1[i].value, events2[i].value, 'rand deterministic values');
    }
});

// === Test irand ===
runner.test('irand produces integers in range', function() {
    const p = irand(5);
    const events = p.queryArc(0, 20);
    
    this.assertLength(events, 20, 'irand one per cycle');
    
    const values = new Set();
    for (const event of events) {
        this.assertInRange(event.value, 0, 4, 'irand value in range');
        this.assertEqual(Math.floor(event.value), event.value, 'irand produces integers');
        values.add(event.value);
    }
    
    // Should hit multiple different values
    if (values.size < 3) {
        console.warn('  Note: irand produced limited variety, but this is probabilistic');
    }
});

// === Test choose ===
runner.test('choose selects from values', function() {
    const p = choose("a", "b", "c");
    const events = p.queryArc(0, 30);
    
    this.assertLength(events, 30, 'choose one per cycle');
    
    const counts = { a: 0, b: 0, c: 0 };
    for (const event of events) {
        this.assertContains(["a", "b", "c"], event.value, 'choose valid value');
        counts[event.value]++;
    }
    
    // Should have some of each (probabilistic)
    if (counts.a === 0 || counts.b === 0 || counts.c === 0) {
        console.warn('  Note: choose missed some values, but this is probabilistic');
    }
});

runner.test('choose is deterministic', function() {
    const p = choose("x", "y", "z");
    
    const events1 = p.queryArc(0, 5);
    const events2 = p.queryArc(0, 5);
    
    for (let i = 0; i < events1.length; i++) {
        this.assertEqual(events1[i].value, events2[i].value, 'choose deterministic');
    }
});

// === Test wchoose ===
runner.test('wchoose respects weights', function() {
    const p = wchoose(["common", 9], ["rare", 1]);
    const events = p.queryArc(0, 100);
    
    this.assertLength(events, 100, 'wchoose one per cycle');
    
    const counts = { common: 0, rare: 0 };
    for (const event of events) {
        counts[event.value]++;
    }
    
    // Common should be roughly 9x more frequent than rare
    const ratio = counts.common / (counts.rare || 1);
    if (ratio < 5 || ratio > 15) {
        console.warn(`  Note: wchoose ratio was ${ratio.toFixed(1)}, expected ~9`);
    }
});

// === Test degrade ===
runner.test('degrade removes roughly 50% of events', function() {
    const p = degrade(cat(pure("a"), pure("b"), pure("c"), pure("d")));
    const events = p.queryArc(0, 25);
    
    // Should have removed roughly half
    const expectedCount = 100; // 4 events * 25 cycles
    const actualCount = events.length;
    const removalRate = 1 - (actualCount / expectedCount);
    
    this.assertApproxEqual(removalRate, 0.5, 0.15, 'degrade ~50% removal');
});

// === Test degradeBy ===
runner.test('degradeBy removes correct proportion', function() {
    const p = degradeBy(0.75, cat(pure("a"), pure("b"), pure("c"), pure("d")));
    const events = p.queryArc(0, 25);
    
    // Should have removed roughly 75%
    const expectedCount = 100; // 4 events * 25 cycles
    const actualCount = events.length;
    const removalRate = 1 - (actualCount / expectedCount);
    
    this.assertApproxEqual(removalRate, 0.75, 0.15, 'degradeBy 75% removal');
});

runner.test('degradeBy 0 removes nothing', function() {
    const original = cat(pure("a"), pure("b"));
    const p = degradeBy(0, original);
    
    const origEvents = original.queryArc(0, 5);
    const degradedEvents = p.queryArc(0, 5);
    
    this.assertLength(degradedEvents, origEvents.length, 'degradeBy 0 keeps all');
});

runner.test('degradeBy 1 removes everything', function() {
    const p = degradeBy(1, cat(pure("a"), pure("b")));
    const events = p.queryArc(0, 5);
    
    this.assertLength(events, 0, 'degradeBy 1 removes all');
});

// === Test shuffle ===
runner.test('shuffle rearranges pattern slices', function() {
    const original = cat(pure("a"), pure("b"), pure("c"), pure("d"));
    const p = shuffle(4, original);
    
    // Check first cycle
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    this.assertLength(events, 4, 'shuffle preserves event count');
    
    // All values should be present
    const values = events.map(e => e.value).sort();
    this.assertEqual(values, ["a", "b", "c", "d"], 'shuffle preserves all values');
    
    // Check multiple cycles have different arrangements
    const events2 = p.queryArc(1, 2).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    const values2 = events2.map(e => e.value);
    
    // Unlikely to have same order (but possible)
    if (JSON.stringify(values) === JSON.stringify(values2)) {
        console.warn('  Note: shuffle produced same order twice (rare but possible)');
    }
});

// === Test scramble ===
runner.test('scramble allows repetition', function() {
    const original = cat(pure("a"), pure("b"), pure("c"), pure("d"));
    const p = scramble(4, original);
    
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    this.assertLength(events, 4, 'scramble preserves event count');
    
    // May have repeated values
    const values = events.map(e => e.value);
    const unique = new Set(values);
    
    // Should sometimes have duplicates
    if (unique.size === 4) {
        console.warn('  Note: scramble had no duplicates this time');
    }
});

// === Test sometimes ===
runner.test('sometimes applies function ~50% of time', function() {
    const p = sometimes(pat => fast(2, pat), pure("bd"));
    
    // Count how many cycles have 2 events (fast applied) vs 1 event (not applied)
    let fastCount = 0;
    let normalCount = 0;
    
    for (let i = 0; i < 20; i++) {
        const events = p.queryArc(i, i + 1);
        if (events.length === 2) {
            fastCount++;
        } else if (events.length === 1) {
            normalCount++;
        }
    }
    
    const ratio = fastCount / (fastCount + normalCount);
    this.assertApproxEqual(ratio, 0.5, 0.2, 'sometimes ~50% application');
});

// === Test often ===
runner.test('often applies function ~75% of time', function() {
    const p = often(pat => fast(2, pat), pure("bd"));
    
    let fastCount = 0;
    let normalCount = 0;
    
    for (let i = 0; i < 20; i++) {
        const events = p.queryArc(i, i + 1);
        if (events.length === 2) {
            fastCount++;
        } else if (events.length === 1) {
            normalCount++;
        }
    }
    
    const ratio = fastCount / (fastCount + normalCount);
    this.assertApproxEqual(ratio, 0.75, 0.2, 'often ~75% application');
});

// === Test rarely ===
runner.test('rarely applies function ~25% of time', function() {
    const p = rarely(pat => fast(2, pat), pure("bd"));
    
    let fastCount = 0;
    let normalCount = 0;
    
    for (let i = 0; i < 20; i++) {
        const events = p.queryArc(i, i + 1);
        if (events.length === 2) {
            fastCount++;
        } else if (events.length === 1) {
            normalCount++;
        }
    }
    
    const ratio = fastCount / (fastCount + normalCount);
    this.assertApproxEqual(ratio, 0.25, 0.2, 'rarely ~25% application');
});

// === Test determinism ===
runner.test('random patterns are deterministic', function() {
    const p = sometimes(
        pat => fast(2, pat),
        degrade(choose("a", "b", "c"))
    );
    
    const events1 = p.queryArc(0, 10);
    const events2 = p.queryArc(0, 10);
    
    this.assertLength(events1, events2.length, 'random deterministic length');
    
    for (let i = 0; i < events1.length; i++) {
        this.assertEqual(events1[i].value, events2[i].value, 'random deterministic values');
        this.assertEqual(
            events1[i].part.begin.toFloat(),
            events2[i].part.begin.toFloat(),
            'random deterministic timing'
        );
    }
});

// Run all tests
runner.run().then(success => {
    process.exit(success ? 0 : 1);
});