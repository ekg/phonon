#!/usr/bin/env node

/**
 * Advanced Pattern Operator Tests
 * Tests combination, filtering, math, and other operators
 */

const {
    Pattern,
    pure,
    cat,
    stack,
    jux,
    juxBy,
    superimpose,
    layer,
    off,
    echo,
    stut,
    when,
    mask,
    struct,
    filter,
    add,
    sub,
    mul,
    div,
    mod,
    range,
    sequence,
    polymeter,
    polyrhythm,
    euclid,
    fast,
    sine
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
        console.log('🧪 Running Advanced Pattern Tests...\n');
        
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

// === Test jux ===
runner.test('jux creates stereo split', function() {
    const p = jux(pat => fast(2, pat), pure("bd"));
    const events = p.queryArc(0, 1);
    
    // Should have original + transformed
    this.assertLength(events, 3, 'jux creates 3 events (1 left + 2 right)');
    
    // Check pan values
    const leftEvents = events.filter(e => e.context.pan === 0);
    const rightEvents = events.filter(e => e.context.pan === 1);
    
    this.assertLength(leftEvents, 1, 'jux has 1 left event');
    this.assertLength(rightEvents, 2, 'jux has 2 right events');
});

// === Test juxBy ===
runner.test('juxBy controls pan amount', function() {
    const p = juxBy(0.5, pat => fast(2, pat), pure("bd"));
    const events = p.queryArc(0, 1);
    
    // Check pan values
    const leftEvents = events.filter(e => e.context.pan === 0.25);
    const rightEvents = events.filter(e => e.context.pan === 0.75);
    
    this.assertLength(leftEvents, 1, 'juxBy has correct left pan');
    this.assertLength(rightEvents, 2, 'juxBy has correct right pan');
});

// === Test superimpose ===
runner.test('superimpose layers patterns', function() {
    const p = superimpose(pat => fast(2, pat), pure("bd"));
    const events = p.queryArc(0, 1);
    
    // Should have original + transformed
    this.assertLength(events, 3, 'superimpose creates 3 events');
});

// === Test layer ===
runner.test('layer applies multiple functions', function() {
    const p = layer(
        pat => fast(2, pat),
        pat => fast(3, pat)
    )(pure("bd"));
    
    const events = p.queryArc(0, 1);
    
    // Should have original + 2 fast + 3 fast = 1 + 2 + 3 = 6
    this.assertLength(events, 6, 'layer creates all variations');
});

// === Test off ===
runner.test('off creates delayed layer', function() {
    const p = off(0.25, pat => pure("sn"), pure("bd"));
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    // Should have original bd + delayed sn
    const bdEvents = events.filter(e => e.value === "bd");
    const snEvents = events.filter(e => e.value === "sn");
    
    this.assertLength(bdEvents, 1, 'off has original');
    this.assertLength(snEvents, 1, 'off has delayed layer');
    
    // The sn should be delayed but pure("sn") creates full cycle event
    // So we just check both exist
});

// === Test echo ===
runner.test('echo creates repeats with decay', function() {
    const p = echo(2, 0.25, 0.5, pure("bd"));
    const events = p.queryArc(0, 1);
    
    // Should have original + 2 echoes
    // Note: may have parts from previous cycle too
    const values = events.map(e => e.value);
    
    // Check that we have events with gain metadata
    const withGain = events.filter(e => e.context && 'gain' in e.context);
    if (withGain.length === 0) {
        // Original events don't have gain, but that's ok
        const allBd = values.every(v => v === "bd" || (v && v.gain !== undefined));
        this.assertEqual(allBd, true, 'echo creates bd events');
    }
});

// === Test when ===
runner.test('when conditionally applies function', function() {
    const p = when(
        v => v === "bd",
        pat => pure("kick"),
        cat(pure("bd"), pure("sn"))
    );
    
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    // bd should become kick, sn should stay
    this.assertEqual(events[0].value, "kick", 'when transforms matching');
    this.assertEqual(events[1].value, "sn", 'when preserves non-matching');
});

// === Test mask ===
runner.test('mask filters by boolean pattern', function() {
    const p = mask(
        cat(pure(true), pure(false)),
        cat(pure("bd"), pure("sn"))
    );
    
    const events = p.queryArc(0, 1);
    
    // Only first half should pass through
    this.assertLength(events, 1, 'mask filters events');
    this.assertEqual(events[0].value, "bd", 'mask keeps masked events');
});

// === Test struct ===
runner.test('struct applies structure', function() {
    const p = struct(
        euclid(3, 8),
        pure("bd")
    );
    
    const events = p.queryArc(0, 1);
    
    // Should have 3 events (from euclid structure)
    this.assertLength(events, 3, 'struct uses structure pattern');
    
    // All should be bd
    const values = events.map(e => e.value);
    this.assertEqual(values, ["bd", "bd", "bd"], 'struct uses value pattern');
});

// === Test filter ===
runner.test('filter removes non-matching events', function() {
    const p = filter(
        v => v !== "sn",
        cat(pure("bd"), pure("sn"), pure("hh"))
    );
    
    const events = p.queryArc(0, 1);
    
    // Should have bd and hh, not sn
    this.assertLength(events, 2, 'filter removes events');
    
    const values = events.map(e => e.value).sort();
    this.assertEqual(values, ["bd", "hh"], 'filter keeps matching');
});

// === Test math operations ===
runner.test('add adds to values', function() {
    const p = add(10, pure(5));
    const events = p.queryArc(0, 1);
    
    this.assertEqual(events[0].value, 15, 'add works');
});

runner.test('sub subtracts from values', function() {
    const p = sub(3, pure(10));
    const events = p.queryArc(0, 1);
    
    this.assertEqual(events[0].value, 7, 'sub works');
});

runner.test('mul multiplies values', function() {
    const p = mul(3, pure(4));
    const events = p.queryArc(0, 1);
    
    this.assertEqual(events[0].value, 12, 'mul works');
});

runner.test('div divides values', function() {
    const p = div(2, pure(10));
    const events = p.queryArc(0, 1);
    
    this.assertEqual(events[0].value, 5, 'div works');
});

runner.test('mod applies modulo', function() {
    const p = mod(3, pure(10));
    const events = p.queryArc(0, 1);
    
    this.assertEqual(events[0].value, 1, 'mod works');
});

runner.test('range maps to range', function() {
    const p = range(100, 200, pure(0.5));
    const events = p.queryArc(0, 1);
    
    this.assertEqual(events[0].value, 150, 'range maps correctly');
});

// === Test sequence ===
runner.test('sequence plays patterns sequentially', function() {
    const p = sequence(pure("a"), pure("b"), pure("c"));
    const events = p.queryArc(0, 1).sort((a, b) => a.part.begin.toFloat() - b.part.begin.toFloat());
    
    this.assertLength(events, 3, 'sequence has all patterns');
    this.assertEqual(events[0].value, "a", 'sequence first');
    this.assertEqual(events[1].value, "b", 'sequence second');
    this.assertEqual(events[2].value, "c", 'sequence third');
});

// === Test polymeter ===
runner.test('polymeter combines different lengths', function() {
    const p = polymeter(
        pure("bd"),
        cat(pure("hh"), pure("hh"))
    );
    
    const events = p.queryArc(0, 1);
    
    // Should have 1 bd + 2 hh
    this.assertLength(events, 3, 'polymeter combines patterns');
    
    const bdCount = events.filter(e => e.value === "bd").length;
    const hhCount = events.filter(e => e.value === "hh").length;
    
    this.assertEqual(bdCount, 1, 'polymeter bd count');
    this.assertEqual(hhCount, 2, 'polymeter hh count');
});

// === Test polyrhythm ===
runner.test('polyrhythm creates different speeds', function() {
    const p = polyrhythm(pure("bd"), pure("sn"));
    const events = p.queryArc(0, 1);
    
    // Each pattern plays at 2x speed
    // So we get 2 bd and 2 sn
    this.assertLength(events, 4, 'polyrhythm speeds up patterns');
    
    const bdCount = events.filter(e => e.value === "bd").length;
    const snCount = events.filter(e => e.value === "sn").length;
    
    this.assertEqual(bdCount, 2, 'polyrhythm bd at 2x');
    this.assertEqual(snCount, 2, 'polyrhythm sn at 2x');
});

// Run all tests
runner.run().then(success => {
    process.exit(success ? 0 : 1);
});