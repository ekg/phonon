#!/usr/bin/env node

/**
 * Additional Pattern Operator Tests
 */

const {
    Pattern,
    pure,
    cat,
    fast,
    firstOf,
    lastOf,
    brak,
    press,
    hurry,
    bite,
    striate,
    inhabit,
    arp,
    arpWith,
    rangex,
    fit,
    take,
    drop,
    run,
    steps,
    euclid,
    segment
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

    assertClose(actual, expected, tolerance = 0.001, message = '') {
        const a = typeof actual === 'number' ? actual : actual.toFloat();
        const e = typeof expected === 'number' ? expected : expected.toFloat();
        
        if (Math.abs(a - e) > tolerance) {
            throw new Error(`Assertion failed: ${message}
                Expected: ${e} ± ${tolerance}
                Actual: ${a}`);
        }
    }

    assertHasProperty(obj, prop, message = '') {
        if (!(prop in obj)) {
            throw new Error(`Property assertion failed: ${message}
                Object should have property: ${prop}`);
        }
    }

    async run() {
        console.log('🧪 Running Additional Pattern Tests...\n');
        
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

// === Test firstOf ===
runner.test('firstOf applies function on first cycle', function() {
    const p = firstOf(3, pat => fast(2, pat), pure("bd"));
    
    // Cycle 0: fast (first of 3)
    const events0 = p.queryArc(0, 1);
    this.assertLength(events0, 2, 'firstOf cycle 0 fast');
    
    // Cycle 1: normal
    const events1 = p.queryArc(1, 2);
    this.assertLength(events1, 1, 'firstOf cycle 1 normal');
    
    // Cycle 2: normal
    const events2 = p.queryArc(2, 3);
    this.assertLength(events2, 1, 'firstOf cycle 2 normal');
    
    // Cycle 3: fast again (first of next group)
    const events3 = p.queryArc(3, 4);
    this.assertLength(events3, 2, 'firstOf cycle 3 fast again');
});

// === Test lastOf ===
runner.test('lastOf applies function on last cycle', function() {
    const p = lastOf(3, pat => fast(2, pat), pure("bd"));
    
    // Cycle 0: normal
    const events0 = p.queryArc(0, 1);
    this.assertLength(events0, 1, 'lastOf cycle 0 normal');
    
    // Cycle 1: normal
    const events1 = p.queryArc(1, 2);
    this.assertLength(events1, 1, 'lastOf cycle 1 normal');
    
    // Cycle 2: fast (last of 3)
    const events2 = p.queryArc(2, 3);
    this.assertLength(events2, 2, 'lastOf cycle 2 fast');
});

// === Test brak ===
runner.test('brak creates syncopated pattern', function() {
    const p = brak(pure("bd"));
    
    // Even cycle: normal
    const events0 = p.queryArc(0, 1);
    this.assertLength(events0, 1, 'brak even cycle');
    this.assertClose(events0[0].part.begin.toFloat(), 0, 0.001, 'brak even starts at 0');
    
    // Odd cycle: shifted
    const events1 = p.queryArc(1, 2);
    this.assertLength(events1, 2, 'brak odd cycle has overlap');
    // Should have parts from shifted pattern
});

// === Test press ===
runner.test('press compresses to start', function() {
    const p = press(pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 1, 'press single event');
    this.assertClose(events[0].part.begin.toFloat(), 0, 0.001, 'press starts at 0');
    this.assertClose(events[0].part.end.toFloat(), 0.5, 0.001, 'press ends at 0.5');
});

// === Test hurry ===
runner.test('hurry speeds up with pitch', function() {
    const p = hurry(2, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'hurry speeds up');
    
    // Check for speed metadata
    if (typeof events[0].value === 'object') {
        this.assertHasProperty(events[0].value, 'speed', 'hurry adds speed metadata');
    }
});

// === Test bite ===
runner.test('bite takes specific slice', function() {
    const p = bite(4, 1, cat(pure("a"), pure("b"), pure("c"), pure("d")));
    const events = p.queryArc(0, 1);
    
    // Should only have the second slice (index 1)
    this.assertLength(events, 1, 'bite single slice');
    this.assertEqual(events[0].value, "b", 'bite correct slice');
});

// === Test striate ===
runner.test('striate interleaves slices', function() {
    const p = striate(2, cat(pure("a"), pure("b")));
    const events = p.queryArc(0, 1);
    
    // Should have 4 events (2 from each, interleaved)
    this.assertLength(events, 4, 'striate creates interleaved events');
});

// === Test arp ===
runner.test('arp up creates ascending arpeggio', function() {
    const p = arp('up', pure(["c", "e", "g"]));
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 3, 'arp creates 3 notes');
    this.assertEqual(events[0].value, "c", 'arp first note');
    this.assertEqual(events[1].value, "e", 'arp second note');
    this.assertEqual(events[2].value, "g", 'arp third note');
});

runner.test('arp down creates descending arpeggio', function() {
    const p = arp('down', pure(["c", "e", "g"]));
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 3, 'arp down creates 3 notes');
    this.assertEqual(events[0].value, "g", 'arp down first note');
    this.assertEqual(events[1].value, "e", 'arp down second note');
    this.assertEqual(events[2].value, "c", 'arp down third note');
});

// === Test arpWith ===
runner.test('arpWith uses custom function', function() {
    const p = arpWith(vals => vals.map(v => v + "!"), pure(["a", "b", "c"]));
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 3, 'arpWith custom');
    this.assertEqual(events[0].value, "a!", 'arpWith transformed');
});

// === Test rangex ===
runner.test('rangex maps exponentially', function() {
    const p = rangex(100, 1000, pure(0.5));
    const events = p.queryArc(0, 1);
    
    const value = events[0].value;
    // Should be somewhere in middle but not linear
    this.assertEqual(value > 100 && value < 1000, true, 'rangex in range');
    // For exponential, 0.5 should map to less than linear midpoint
    this.assertEqual(value < 550, true, 'rangex exponential curve');
});

// === Test fit ===
runner.test('fit adjusts to n steps', function() {
    const p = fit(4, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 4, 'fit creates 4 events');
});

// === Test take ===
runner.test('take keeps first n events', function() {
    const p = take(2, cat(pure("a"), pure("b"), pure("c"), pure("d")));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'take first 2');
    const values = events.map(e => e.value).sort();
    this.assertEqual(values, ["a", "b"], 'take correct events');
});

// === Test drop ===
runner.test('drop removes first n events', function() {
    const p = drop(2, cat(pure("a"), pure("b"), pure("c"), pure("d")));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'drop first 2');
    const values = events.map(e => e.value).sort();
    this.assertEqual(values, ["c", "d"], 'drop correct events');
});

// === Test run ===
runner.test('run creates number sequence', function() {
    const p = run(4);
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 4, 'run 4 numbers');
    this.assertEqual(events[0].value, 0, 'run starts at 0');
    this.assertEqual(events[1].value, 1, 'run continues');
    this.assertEqual(events[2].value, 2, 'run continues');
    this.assertEqual(events[3].value, 3, 'run ends at n-1');
});

// === Test steps ===
runner.test('steps sets step count', function() {
    const p = steps(8, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 8, 'steps creates 8 events');
});

// === Test inhabit ===
runner.test('inhabit fills structure', function() {
    const p = inhabit(euclid(3, 8), pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 3, 'inhabit uses structure');
    const values = events.map(e => e.value);
    this.assertEqual(values, ["bd", "bd", "bd"], 'inhabit fills with value');
});

// Run all tests
runner.run().then(success => {
    process.exit(success ? 0 : 1);
});