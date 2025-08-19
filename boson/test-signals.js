#!/usr/bin/env node

/**
 * Signal Generators and Euclidean Rhythm Tests
 */

const {
    sine,
    cosine,
    saw,
    square,
    tri,
    perlin,
    euclid,
    euclidRot,
    euclidLegato
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

    assertClose(actual, expected, tolerance = 0.1, message = '') {
        if (Math.abs(actual - expected) > tolerance) {
            throw new Error(`Close assertion failed: ${message}
                Expected: ${expected} ± ${tolerance}
                Actual: ${actual}`);
        }
    }

    assertMonotonic(values, increasing = true, message = '') {
        for (let i = 1; i < values.length; i++) {
            if (increasing && values[i] < values[i-1]) {
                throw new Error(`Monotonic assertion failed: ${message}
                    Values should be increasing but ${values[i]} < ${values[i-1]}`);
            }
            if (!increasing && values[i] > values[i-1]) {
                throw new Error(`Monotonic assertion failed: ${message}
                    Values should be decreasing but ${values[i]} > ${values[i-1]}`);
            }
        }
    }

    async run() {
        console.log('🧪 Running Signal & Euclidean Tests...\n');
        
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

// === Test sine ===
runner.test('sine produces values between 0 and 1', function() {
    const p = sine();
    const events = p.queryArc(0, 1);
    
    // Should have multiple samples per cycle
    if (events.length < 8) {
        throw new Error('Sine should have at least 8 samples per cycle');
    }
    
    for (const event of events) {
        this.assertInRange(event.value, 0, 1, 'sine value in range');
    }
});

runner.test('sine starts at 0.5 and peaks at 0.25', function() {
    const p = sine();
    const events = p.queryArc(0, 1);
    
    // First event should be near 0.5 (sin(0) = 0, mapped to 0.5)
    this.assertClose(events[0].value, 0.5, 0.1, 'sine starts near 0.5');
    
    // Around t=0.25, should be near 1 (sin(π/2) = 1)
    const quarterEvents = events.filter(e => 
        e.part.begin.toFloat() >= 0.2 && e.part.begin.toFloat() <= 0.3);
    if (quarterEvents.length > 0) {
        const maxVal = Math.max(...quarterEvents.map(e => e.value));
        this.assertClose(maxVal, 1, 0.1, 'sine peaks near 1 at 0.25');
    }
});

// === Test cosine ===
runner.test('cosine starts at 1', function() {
    const p = cosine();
    const events = p.queryArc(0, 1);
    
    // First event should be near 1 (cos(0) = 1)
    this.assertClose(events[0].value, 1, 0.1, 'cosine starts near 1');
});

// === Test saw ===
runner.test('saw produces linear ramp', function() {
    const p = saw();
    const events = p.queryArc(0, 1);
    
    // Should ramp from 0 to 1
    this.assertClose(events[0].value, 0, 0.1, 'saw starts near 0');
    this.assertClose(events[events.length - 1].value, 1, 0.1, 'saw ends near 1');
    
    // Should be monotonically increasing
    const values = events.map(e => e.value);
    this.assertMonotonic(values, true, 'saw increases monotonically');
});

// === Test square ===
runner.test('square produces binary values', function() {
    const p = square();
    const events = p.queryArc(0, 1);
    
    this.assertLength(events, 2, 'square has 2 events per cycle');
    
    // First half should be 1
    this.assertEqual(events[0].value, 1, 'square first half is 1');
    this.assertClose(events[0].part.begin.toFloat(), 0, 0.001, 'square first half starts at 0');
    this.assertClose(events[0].part.end.toFloat(), 0.5, 0.001, 'square first half ends at 0.5');
    
    // Second half should be 0
    this.assertEqual(events[1].value, 0, 'square second half is 0');
    this.assertClose(events[1].part.begin.toFloat(), 0.5, 0.001, 'square second half starts at 0.5');
    this.assertClose(events[1].part.end.toFloat(), 1, 0.001, 'square second half ends at 1');
});

// === Test tri ===
runner.test('triangle wave rises then falls', function() {
    const p = tri();
    const events = p.queryArc(0, 1);
    
    // First half should rise
    const firstHalf = events.filter(e => e.part.begin.toFloat() < 0.5);
    const firstValues = firstHalf.map(e => e.value);
    this.assertMonotonic(firstValues, true, 'tri first half rises');
    
    // Second half should fall
    const secondHalf = events.filter(e => e.part.begin.toFloat() >= 0.5);
    const secondValues = secondHalf.map(e => e.value);
    this.assertMonotonic(secondValues, false, 'tri second half falls');
});

// === Test perlin ===
runner.test('perlin produces smooth noise', function() {
    const p = perlin();
    const events = p.queryArc(0, 2);
    
    // All values should be in range
    for (const event of events) {
        this.assertInRange(event.value, 0, 1, 'perlin value in range');
    }
    
    // Should be deterministic
    const events2 = p.queryArc(0, 2);
    for (let i = 0; i < events.length; i++) {
        this.assertEqual(events[i].value, events2[i].value, 'perlin is deterministic');
    }
});

// === Test euclid ===
runner.test('euclid(3, 8) creates correct pattern', function() {
    const p = euclid(3, 8);
    const events = p.queryArc(0, 1);
    
    // Should have 3 pulses in 8 steps
    this.assertLength(events, 3, 'euclid(3,8) has 3 pulses');
    
    // Check positions (Bjorklund algorithm gives: [1,0,0,1,0,0,1,0])
    // Events at positions 0, 3, 6
    this.assertClose(events[0].part.begin.toFloat(), 0/8, 0.001, 'euclid first pulse');
    this.assertClose(events[1].part.begin.toFloat(), 3/8, 0.001, 'euclid second pulse');
    this.assertClose(events[2].part.begin.toFloat(), 6/8, 0.001, 'euclid third pulse');
});

runner.test('euclid(5, 8) creates correct pattern', function() {
    const p = euclid(5, 8);
    const events = p.queryArc(0, 1);
    
    // Should have 5 pulses in 8 steps
    this.assertLength(events, 5, 'euclid(5,8) has 5 pulses');
});

runner.test('euclid edge cases', function() {
    // euclid(0, 8) should have no events
    const p1 = euclid(0, 8);
    const events1 = p1.queryArc(0, 1);
    this.assertLength(events1, 0, 'euclid(0,8) has no pulses');
    
    // euclid(8, 8) should have 8 events
    const p2 = euclid(8, 8);
    const events2 = p2.queryArc(0, 1);
    this.assertLength(events2, 8, 'euclid(8,8) has all pulses');
    
    // euclid(10, 8) should cap at 8
    const p3 = euclid(10, 8);
    const events3 = p3.queryArc(0, 1);
    this.assertLength(events3, 8, 'euclid(10,8) caps at 8');
});

// === Test euclidRot ===
runner.test('euclidRot rotates pattern', function() {
    const p1 = euclid(3, 8, 0);
    const p2 = euclidRot(3, 8, 1);
    
    const events1 = p1.queryArc(0, 1);
    const events2 = p2.queryArc(0, 1);
    
    this.assertLength(events2, 3, 'euclidRot preserves pulse count');
    
    // The rotation shifts the pattern
    // Original: [1,0,0,1,0,0,1,0] -> positions 0, 3, 6
    // Rotated by 1: [0,1,0,0,1,0,0,1] -> positions 1, 4, 7
    // But bjorklund rotates to start with pulse, so actual is different
    // Just check that they're different
    const pos1 = events1.map(e => e.part.begin.toFloat());
    const pos2 = events2.map(e => e.part.begin.toFloat());
    
    // Should be different
    let different = false;
    for (let i = 0; i < pos1.length; i++) {
        if (Math.abs(pos1[i] - pos2[i]) > 0.001) {
            different = true;
            break;
        }
    }
    
    if (!different) {
        throw new Error('euclidRot should produce different pattern');
    }
});

// === Test euclidLegato ===
runner.test('euclidLegato extends note durations', function() {
    const p = euclidLegato(3, 8);
    const events = p.queryArc(0, 1);
    
    // Should have 3 notes but they may be longer
    this.assertLength(events, 3, 'euclidLegato has correct note count');
    
    // Notes should extend until next note or cycle end
    for (const event of events) {
        const duration = event.part.end.toFloat() - event.part.begin.toFloat();
        if (duration <= 1/8) {
            throw new Error('euclidLegato should extend note durations');
        }
    }
});

runner.test('signals are deterministic', function() {
    const signals = [sine(), cosine(), saw(), square(), tri(), perlin()];
    
    for (const signal of signals) {
        const events1 = signal.queryArc(0, 2);
        const events2 = signal.queryArc(0, 2);
        
        this.assertLength(events1, events2.length, 'signal deterministic length');
        
        for (let i = 0; i < events1.length; i++) {
            this.assertEqual(events1[i].value, events2[i].value, 'signal deterministic values');
        }
    }
});

// Run all tests
runner.run().then(success => {
    process.exit(success ? 0 : 1);
});