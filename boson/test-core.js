#!/usr/bin/env node

/**
 * Core Pattern Operator Tests
 * Tests the fundamental pattern operators
 */

const {
    Pattern,
    Fraction,
    TimeSpan,
    Event,
    pure,
    silence,
    gap,
    stack,
    cat,
    fastcat,
    slowcat,
    fast,
    slow
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

    assertEventAt(events, index, expectedStart, expectedEnd, expectedValue, message = '') {
        if (index >= events.length) {
            throw new Error(`${message}: No event at index ${index} (only ${events.length} events)`);
        }
        
        const event = events[index];
        this.assertClose(event.part.begin.toFloat(), expectedStart, 0.001, 
            `${message}: Event ${index} start time`);
        this.assertClose(event.part.end.toFloat(), expectedEnd, 0.001,
            `${message}: Event ${index} end time`);
        
        if (expectedValue !== undefined) {
            this.assertEqual(event.value, expectedValue,
                `${message}: Event ${index} value`);
        }
    }

    async run() {
        console.log('🧪 Running Core Pattern Tests...\n');
        
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

// === Test Fraction class ===
runner.test('Fraction arithmetic', function() {
    const a = new Fraction(1, 2);
    const b = new Fraction(1, 3);
    
    this.assertClose(a.add(b).toFloat(), 5/6, 0.001, 'Addition');
    this.assertClose(a.sub(b).toFloat(), 1/6, 0.001, 'Subtraction');
    this.assertClose(a.mul(b).toFloat(), 1/6, 0.001, 'Multiplication');
    this.assertClose(a.div(b).toFloat(), 3/2, 0.001, 'Division');
});

// === Test TimeSpan class ===
runner.test('TimeSpan operations', function() {
    const a = new TimeSpan(0, 1);
    const b = new TimeSpan(0.5, 1.5);
    
    const intersection = a.intersection(b);
    this.assertClose(intersection.begin.toFloat(), 0.5, 0.001, 'Intersection begin');
    this.assertClose(intersection.end.toFloat(), 1, 0.001, 'Intersection end');
    
    this.assertEqual(a.overlaps(b), true, 'Overlaps');
    this.assertEqual(a.contains(0.5), true, 'Contains 0.5');
    this.assertEqual(a.contains(1), false, 'Does not contain 1');
});

// === Test pure ===
runner.test('pure creates constant pattern', function() {
    const p = pure(42);
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 1, 'pure single cycle');
    this.assertEventAt(events, 0, 0, 1, 42, 'pure value');
    
    // Test multiple cycles
    const events2 = p.queryArc(0, 2);
    this.assertLength(events2, 2, 'pure two cycles');
    this.assertEventAt(events2, 0, 0, 1, 42, 'pure first cycle');
    this.assertEventAt(events2, 1, 1, 2, 42, 'pure second cycle');
});

// === Test silence ===
runner.test('silence creates empty pattern', function() {
    const p = silence();
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 0, 'silence has no events');
    
    const events2 = p.queryArc(0, 10);
    this.assertLength(events2, 0, 'silence has no events over 10 cycles');
});

// === Test gap ===
runner.test('gap creates pattern with gaps', function() {
    const p = gap(2);
    const events = p.queryArc(0, 4);
    
    this.assertLength(events, 2, 'gap every 2 cycles');
    this.assertEventAt(events, 0, 0, 1, null, 'gap at cycle 0');
    this.assertEventAt(events, 1, 2, 3, null, 'gap at cycle 2');
});

// === Test stack ===
runner.test('stack combines patterns in parallel', function() {
    const p = stack(pure("bd"), pure("hh"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'stack has both events');
    
    // Check both events exist (order may vary)
    const values = events.map(e => e.value).sort();
    this.assertEqual(values, ["bd", "hh"], 'stack values');
    
    // Both should span full cycle
    for (const event of events) {
        this.assertClose(event.part.begin.toFloat(), 0, 0.001, 'stack event start');
        this.assertClose(event.part.end.toFloat(), 1, 0.001, 'stack event end');
    }
});

// === Test cat ===
runner.test('cat sequences patterns in one cycle', function() {
    const p = cat(pure("bd"), pure("sn"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'cat has both events');
    this.assertEventAt(events, 0, 0, 0.5, "bd", 'cat first half');
    this.assertEventAt(events, 1, 0.5, 1, "sn", 'cat second half');
});

runner.test('cat with three patterns', function() {
    const p = cat(pure("bd"), pure("sn"), pure("hh"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 3, 'cat three patterns');
    this.assertEventAt(events, 0, 0, 1/3, "bd", 'cat first third');
    this.assertEventAt(events, 1, 1/3, 2/3, "sn", 'cat second third');
    this.assertEventAt(events, 2, 2/3, 1, "hh", 'cat third third');
});

// === Test fastcat ===
runner.test('fastcat is same as cat', function() {
    const p1 = cat(pure("bd"), pure("sn"));
    const p2 = fastcat(pure("bd"), pure("sn"));
    
    const events1 = p1.queryArc(0, 1);
    const events2 = p2.queryArc(0, 1);
    
    this.assertLength(events2, events1.length, 'fastcat same length as cat');
    
    for (let i = 0; i < events1.length; i++) {
        this.assertClose(events1[i].part.begin.toFloat(), 
                        events2[i].part.begin.toFloat(), 0.001, 'fastcat timing matches cat');
    }
});

// === Test slowcat ===
runner.test('slowcat sequences patterns across cycles', function() {
    const p = slowcat(pure("bd"), pure("sn"), pure("hh"));
    const events = p.queryArc(0, 3);
    
    this.assertLength(events, 3, 'slowcat three cycles');
    this.assertEventAt(events, 0, 0, 1, "bd", 'slowcat cycle 0');
    this.assertEventAt(events, 1, 1, 2, "sn", 'slowcat cycle 1');
    this.assertEventAt(events, 2, 2, 3, "hh", 'slowcat cycle 2');
    
    // Test wraparound
    const events2 = p.queryArc(3, 4);
    this.assertLength(events2, 1, 'slowcat wraps around');
    this.assertEventAt(events2, 0, 3, 4, "bd", 'slowcat wraps to first pattern');
});

// === Test fast ===
runner.test('fast speeds up pattern by factor', function() {
    const p = fast(2, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'fast 2 creates 2 events');
    this.assertEventAt(events, 0, 0, 0.5, "bd", 'fast first half');
    this.assertEventAt(events, 1, 0.5, 1, "bd", 'fast second half');
    
    // Test fast 4
    const p2 = fast(4, pure("bd"));
    const events2 = p2.queryArc(0, 1);
    this.assertLength(events2, 4, 'fast 4 creates 4 events');
});

runner.test('fast on cat pattern', function() {
    const p = fast(2, cat(pure("bd"), pure("sn")));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 4, 'fast 2 on cat creates 4 events');
    this.assertEventAt(events, 0, 0, 0.25, "bd", 'fast cat 1st quarter');
    this.assertEventAt(events, 1, 0.25, 0.5, "sn", 'fast cat 2nd quarter');
    this.assertEventAt(events, 2, 0.5, 0.75, "bd", 'fast cat 3rd quarter');
    this.assertEventAt(events, 3, 0.75, 1, "sn", 'fast cat 4th quarter');
});

// === Test slow ===
runner.test('slow slows down pattern by factor', function() {
    const p = slow(2, cat(pure("bd"), pure("sn")));
    const events = p.queryArc(0, 2);
    
    this.assertLength(events, 2, 'slow 2 stretches over 2 cycles');
    this.assertEventAt(events, 0, 0, 1, "bd", 'slow first cycle');
    this.assertEventAt(events, 1, 1, 2, "sn", 'slow second cycle');
});

runner.test('slow 4 on pure', function() {
    const p = slow(4, pure("bd"));
    const events = p.queryArc(0, 4);
    
    this.assertLength(events, 1, 'slow 4 creates one long event');
    this.assertEventAt(events, 0, 0, 4, "bd", 'slow 4 spans 4 cycles');
});

// === Test pattern composition ===
runner.test('complex pattern composition', function() {
    // stack(fast(2, pure("bd")), cat(pure("hh"), pure("sn")))
    const p = stack(
        fast(2, pure("bd")),
        cat(pure("hh"), pure("sn"))
    );
    
    const events = p.queryArc(0, 1);
    
    // Should have 2 bd events and 2 other events
    this.assertLength(events, 4, 'complex pattern event count');
    
    const bdEvents = events.filter(e => e.value === "bd");
    const hhEvents = events.filter(e => e.value === "hh");
    const snEvents = events.filter(e => e.value === "sn");
    
    this.assertLength(bdEvents, 2, 'two bd events');
    this.assertLength(hhEvents, 1, 'one hh event');
    this.assertLength(snEvents, 1, 'one sn event');
});

// === Test determinism ===
runner.test('patterns are deterministic', function() {
    const p = stack(
        fast(3, cat(pure("bd"), pure("sn"))),
        slow(2, pure("hh"))
    );
    
    const events1 = p.queryArc(0, 4);
    const events2 = p.queryArc(0, 4);
    
    this.assertLength(events1, events2.length, 'deterministic length');
    
    // Sort events for comparison
    const sort = (events) => events.sort((a, b) => {
        const diff = a.part.begin.toFloat() - b.part.begin.toFloat();
        if (Math.abs(diff) < 0.001) {
            return a.value < b.value ? -1 : 1;
        }
        return diff;
    });
    
    const sorted1 = sort(events1);
    const sorted2 = sort(events2);
    
    for (let i = 0; i < sorted1.length; i++) {
        this.assertClose(sorted1[i].part.begin.toFloat(), 
                        sorted2[i].part.begin.toFloat(), 0.001, 'deterministic timing');
        this.assertEqual(sorted1[i].value, sorted2[i].value, 'deterministic value');
    }
});

// Run all tests
runner.run().then(success => {
    process.exit(success ? 0 : 1);
});