#!/usr/bin/env node

/**
 * Time Manipulation & Structure Pattern Operator Tests
 */

const {
    Pattern,
    pure,
    cat,
    early,
    late,
    compress,
    zoom,
    ply,
    inside,
    outside,
    segment,
    chop,
    rev,
    palindrome,
    iter,
    every,
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
        console.log('🧪 Running Time & Structure Pattern Tests...\n');
        
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

// === Test early ===
runner.test('early shifts pattern earlier', function() {
    const p = early(0.25, pure("bd"));
    const events = p.queryArc(0, 1);
    
    // With early, we get parts of two cycles
    this.assertLength(events, 2, 'early creates two partial events');
    this.assertEventAt(events, 0, 0, 0.75, "bd", 'early current cycle part');
    this.assertEventAt(events, 1, 0.75, 1, "bd", 'early next cycle part');
});

// === Test late ===
runner.test('late shifts pattern later', function() {
    const p = late(0.25, pure("bd"));
    const events = p.queryArc(0, 1);
    
    // With late, we get parts of two cycles
    this.assertLength(events, 2, 'late creates two partial events');
    this.assertEventAt(events, 0, 0, 0.25, "bd", 'late previous cycle part');
    this.assertEventAt(events, 1, 0.25, 1, "bd", 'late current cycle part');
});

// === Test compress ===
runner.test('compress squeezes pattern into timespan', function() {
    const p = compress(0.25, 0.75, cat(pure("bd"), pure("sn")));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'compress two events');
    // Pattern compressed into middle half of cycle
    this.assertEventAt(events, 0, 0.25, 0.5, "bd", 'compress first half');
    this.assertEventAt(events, 1, 0.5, 0.75, "sn", 'compress second half');
});

runner.test('compress to first quarter', function() {
    const p = compress(0, 0.25, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 1, 'compress to quarter');
    this.assertEventAt(events, 0, 0, 0.25, "bd", 'compress quarter');
});

// === Test zoom ===
runner.test('zoom focuses on pattern section', function() {
    const p = zoom(0.25, 0.75, cat(pure("a"), pure("b"), pure("c"), pure("d")));
    const events = p.queryArc(0, 1);
    
    // Should only see the middle section (b and c) stretched to full cycle
    this.assertLength(events, 2, 'zoom middle section');
    this.assertEventAt(events, 0, 0, 0.5, "b", 'zoom first half');
    this.assertEventAt(events, 1, 0.5, 1, "c", 'zoom second half');
});

// === Test ply ===
runner.test('ply repeats each event n times', function() {
    const p = ply(3, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 3, 'ply 3 times');
    this.assertEventAt(events, 0, 0, 1/3, "bd", 'ply first');
    this.assertEventAt(events, 1, 1/3, 2/3, "bd", 'ply second');
    this.assertEventAt(events, 2, 2/3, 1, "bd", 'ply third');
});

runner.test('ply on cat pattern', function() {
    const p = ply(2, cat(pure("bd"), pure("sn")));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 4, 'ply 2 on cat');
    this.assertEventAt(events, 0, 0, 0.25, "bd", 'ply cat 1');
    this.assertEventAt(events, 1, 0.25, 0.5, "bd", 'ply cat 2');
    this.assertEventAt(events, 2, 0.5, 0.75, "sn", 'ply cat 3');
    this.assertEventAt(events, 3, 0.75, 1, "sn", 'ply cat 4');
});

// === Test inside ===
runner.test('inside applies function at higher speed', function() {
    const p = inside(2, (pat) => fast(2, pat), pure("bd"));
    // inside(2, fast(2)) should be like fast(4)
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 4, 'inside fast 2 of fast 2');
    this.assertEventAt(events, 0, 0, 0.25, "bd", 'inside 1');
    this.assertEventAt(events, 1, 0.25, 0.5, "bd", 'inside 2');
    this.assertEventAt(events, 2, 0.5, 0.75, "bd", 'inside 3');
    this.assertEventAt(events, 3, 0.75, 1, "bd", 'inside 4');
});

// === Test outside ===
runner.test('outside applies function at lower speed', function() {
    const p = outside(2, (pat) => slow(2, pat), pure("bd"));
    // outside(2, slow(2)) should be like slow(4)
    const events = p.queryArc(0, 4);
    
    this.assertLength(events, 1, 'outside slow 2 of slow 2');
    this.assertEventAt(events, 0, 0, 4, "bd", 'outside spans 4 cycles');
});

// === Test segment ===
runner.test('segment samples pattern n times per cycle', function() {
    // Create a pattern that changes value over time
    const ramp = new Pattern((span) => {
        const events = [];
        for (let t = Math.floor(span.begin.toFloat()); t < Math.ceil(span.end.toFloat()); t++) {
            const cycleSpan = new Pattern.TimeSpan(t, t + 1);
            const intersection = cycleSpan.intersection(span);
            if (intersection) {
                events.push(new Pattern.Event(cycleSpan, intersection, t));
            }
        }
        return events;
    });
    
    const p = segment(4, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 4, 'segment 4 samples');
    this.assertEventAt(events, 0, 0, 0.25, "bd", 'segment 1');
    this.assertEventAt(events, 1, 0.25, 0.5, "bd", 'segment 2');
    this.assertEventAt(events, 2, 0.5, 0.75, "bd", 'segment 3');
    this.assertEventAt(events, 3, 0.75, 1, "bd", 'segment 4');
});

// === Test chop ===
runner.test('chop divides events into pieces', function() {
    const p = chop(4, pure("bd"));
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 4, 'chop into 4');
    this.assertEventAt(events, 0, 0, 0.25, "bd", 'chop 1');
    this.assertEventAt(events, 1, 0.25, 0.5, "bd", 'chop 2');
    this.assertEventAt(events, 2, 0.5, 0.75, "bd", 'chop 3');
    this.assertEventAt(events, 3, 0.75, 1, "bd", 'chop 4');
    
    // Check metadata
    this.assertClose(events[0].context.chop, 0, 0.001, 'chop position 0');
    this.assertClose(events[1].context.chop, 0.25, 0.001, 'chop position 0.25');
});

// === Test rev ===
runner.test('rev reverses pattern within cycle', function() {
    const p = rev(cat(pure("a"), pure("b"), pure("c")));
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 3, 'rev three events');
    // Should be c, b, a (reversed)
    this.assertEventAt(events, 0, 0, 1/3, "c", 'rev first third');
    this.assertEventAt(events, 1, 1/3, 2/3, "b", 'rev second third');
    this.assertEventAt(events, 2, 2/3, 1, "a", 'rev third third');
});

runner.test('rev is idempotent', function() {
    const original = cat(pure("a"), pure("b"));
    const reversed = rev(rev(original));
    
    const origEvents = original.queryArc(0, 1);
    const revEvents = reversed.queryArc(0, 1);
    
    this.assertLength(revEvents, origEvents.length, 'rev rev length');
    
    for (let i = 0; i < origEvents.length; i++) {
        this.assertClose(origEvents[i].part.begin.toFloat(),
                        revEvents[i].part.begin.toFloat(), 0.001, 'rev rev timing');
        this.assertEqual(origEvents[i].value, revEvents[i].value, 'rev rev value');
    }
});

// === Test palindrome ===
runner.test('palindrome plays forward then backward', function() {
    const p = palindrome(cat(pure("a"), pure("b")));
    const events = p.queryArc(0, 2).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 4, 'palindrome two cycles');
    // First cycle: a, b (forward)
    this.assertEventAt(events, 0, 0, 0.5, "a", 'palindrome cycle 0 first');
    this.assertEventAt(events, 1, 0.5, 1, "b", 'palindrome cycle 0 second');
    // Second cycle: b, a (reversed)
    this.assertEventAt(events, 2, 1, 1.5, "b", 'palindrome cycle 1 first');
    this.assertEventAt(events, 3, 1.5, 2, "a", 'palindrome cycle 1 second');
});

// === Test iter ===
runner.test('iter rotates pattern', function() {
    const p = iter(0.25, cat(pure("a"), pure("b"), pure("c"), pure("d")));
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 4, 'iter four events');
    // First cycle: no rotation (cycle 0 * 0.25 = 0)
    this.assertEventAt(events, 0, 0, 0.25, "a", 'iter first');
    this.assertEventAt(events, 1, 0.25, 0.5, "b", 'iter second');
    this.assertEventAt(events, 2, 0.5, 0.75, "c", 'iter third');
    this.assertEventAt(events, 3, 0.75, 1, "d", 'iter fourth');
    
    // Test second cycle where rotation should happen
    const events2 = p.queryArc(1, 2).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    this.assertLength(events2, 4, 'iter cycle 2 four events');
    // Second cycle: rotated by 0.25 (cycle 1 * 0.25 = 0.25)
    this.assertEventAt(events2, 0, 1, 1.25, "d", 'iter cycle 2 first');
    this.assertEventAt(events2, 1, 1.25, 1.5, "a", 'iter cycle 2 second');
    this.assertEventAt(events2, 2, 1.5, 1.75, "b", 'iter cycle 2 third');
    this.assertEventAt(events2, 3, 1.75, 2, "c", 'iter cycle 2 fourth');
});

// === Test every ===
runner.test('every applies function periodically', function() {
    const p = every(2, (pat) => rev(pat), cat(pure("a"), pure("b")));
    
    // Cycle 0: apply function (reversed) - cycle 0 % 2 == 0
    const events0 = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    this.assertLength(events0, 2, 'every cycle 0');
    this.assertEventAt(events0, 0, 0, 0.5, "b", 'every cycle 0 reversed first');
    this.assertEventAt(events0, 1, 0.5, 1, "a", 'every cycle 0 reversed second');
    
    // Cycle 1: normal - cycle 1 % 2 == 1
    const events1 = p.queryArc(1, 2).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    this.assertLength(events1, 2, 'every cycle 1');
    this.assertEventAt(events1, 0, 1, 1.5, "a", 'every cycle 1 normal first');
    this.assertEventAt(events1, 1, 1.5, 2, "b", 'every cycle 1 normal second');
    
    // Cycle 2: reversed again - cycle 2 % 2 == 0
    const events2 = p.queryArc(2, 3).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    this.assertLength(events2, 2, 'every cycle 2');
    this.assertEventAt(events2, 0, 2, 2.5, "b", 'every cycle 2 reversed first');
    this.assertEventAt(events2, 1, 2.5, 3, "a", 'every cycle 2 reversed second');
});

runner.test('every 3 cycles', function() {
    const p = every(3, (pat) => fast(2, pat), pure("bd"));
    
    // Cycle 0: fast
    const events0 = p.queryArc(0, 1);
    this.assertLength(events0, 2, 'every 3: cycle 0 fast');
    
    // Cycles 1 and 2: normal
    const events1 = p.queryArc(1, 2);
    this.assertLength(events1, 1, 'every 3: cycle 1 normal');
    
    const events2 = p.queryArc(2, 3);
    this.assertLength(events2, 1, 'every 3: cycle 2 normal');
    
    // Cycle 3: fast again
    const events3 = p.queryArc(3, 4);
    this.assertLength(events3, 2, 'every 3: cycle 3 fast');
});

// Run all tests
runner.run().then(success => {
    process.exit(success ? 0 : 1);
});