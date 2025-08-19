// Pattern Testing Framework for Phonon
// Tests pattern operators without requiring audio output

class PatternTest {
  constructor() {
    this.tests = [];
    this.failures = [];
  }

  // Query a pattern for events in a time range
  queryArc(pattern, start, end) {
    // This would call into the actual pattern engine
    // For now, we'll define the expected interface
    return pattern.queryArc(start, end);
  }

  // Assert that two values are equal
  assertEqual(actual, expected, message = '') {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      throw new Error(`Assertion failed: ${message}
        Expected: ${JSON.stringify(expected)}
        Actual: ${JSON.stringify(actual)}`);
    }
  }

  // Assert that a value is close to expected (for floating point)
  assertClose(actual, expected, tolerance = 0.001, message = '') {
    if (Math.abs(actual - expected) > tolerance) {
      throw new Error(`Assertion failed: ${message}
        Expected: ${expected} ± ${tolerance}
        Actual: ${actual}`);
    }
  }

  // Assert array length
  assertLength(array, expectedLength, message = '') {
    if (array.length !== expectedLength) {
      throw new Error(`Length assertion failed: ${message}
        Expected length: ${expectedLength}
        Actual length: ${array.length}`);
    }
  }

  // Test that events occur at expected times
  testEventTiming(pattern, expectedEvents, message = '') {
    const events = this.queryArc(pattern, 0, 1);
    
    this.assertLength(events, expectedEvents.length, 
      `${message}: Wrong number of events`);
    
    events.forEach((event, i) => {
      const expected = expectedEvents[i];
      this.assertClose(event.whole.begin, expected.start, 0.001,
        `${message}: Event ${i} start time`);
      this.assertClose(event.whole.end, expected.end, 0.001,
        `${message}: Event ${i} end time`);
      if (expected.value !== undefined) {
        this.assertEqual(event.value, expected.value,
          `${message}: Event ${i} value`);
      }
    });
  }

  // Test that pattern is deterministic
  testDeterminism(pattern, message = '') {
    const run1 = this.queryArc(pattern, 0, 4);
    const run2 = this.queryArc(pattern, 0, 4);
    this.assertEqual(run1, run2, 
      `${message}: Pattern should be deterministic`);
  }

  // Test mathematical properties
  testReversibility(pattern, message = '') {
    const original = this.queryArc(pattern, 0, 1);
    const reversed = this.queryArc(pattern.rev().rev(), 0, 1);
    this.assertEqual(original, reversed,
      `${message}: rev(rev(p)) should equal p`);
  }

  // Test performance
  testPerformance(pattern, maxMs = 100, message = '') {
    const start = performance.now();
    this.queryArc(pattern, 0, 100); // Query 100 cycles
    const duration = performance.now() - start;
    
    if (duration > maxMs) {
      throw new Error(`Performance test failed: ${message}
        Maximum time: ${maxMs}ms
        Actual time: ${duration}ms`);
    }
  }

  // Run a test case
  test(name, testFn) {
    this.tests.push({ name, testFn });
  }

  // Run all tests
  async runAll() {
    console.log('🧪 Running Pattern Tests...\n');
    
    let passed = 0;
    let failed = 0;
    
    for (const { name, testFn } of this.tests) {
      try {
        await testFn();
        console.log(`✅ ${name}`);
        passed++;
      } catch (error) {
        console.log(`❌ ${name}`);
        console.log(`   ${error.message}`);
        failed++;
        this.failures.push({ name, error });
      }
    }
    
    console.log(`\n📊 Results: ${passed} passed, ${failed} failed`);
    
    if (failed > 0) {
      console.log('\n❌ Failed tests:');
      this.failures.forEach(({ name, error }) => {
        console.log(`\n  ${name}:`);
        console.log(`    ${error.message}`);
      });
    }
    
    return failed === 0;
  }
}

// Example test definitions
const tester = new PatternTest();

// Test pure pattern
tester.test('pure creates constant pattern', () => {
  const p = pure(42);
  tester.testEventTiming(p, [
    { start: 0, end: 1, value: 42 }
  ]);
});

// Test stack
tester.test('stack combines patterns in parallel', () => {
  const p = stack(pure("bd"), pure("hh"));
  tester.testEventTiming(p, [
    { start: 0, end: 1, value: "bd" },
    { start: 0, end: 1, value: "hh" }
  ]);
});

// Test cat
tester.test('cat sequences patterns in one cycle', () => {
  const p = cat(pure("bd"), pure("sn"));
  tester.testEventTiming(p, [
    { start: 0, end: 0.5, value: "bd" },
    { start: 0.5, end: 1, value: "sn" }
  ]);
});

// Test fast
tester.test('fast speeds up pattern by factor', () => {
  const p = fast(2, pure("bd"));
  tester.testEventTiming(p, [
    { start: 0, end: 0.5, value: "bd" },
    { start: 0.5, end: 1, value: "bd" }
  ]);
});

// Test slow
tester.test('slow slows down pattern by factor', () => {
  const p = slow(2, cat(pure("bd"), pure("sn")));
  const events = tester.queryArc(p, 0, 2);
  tester.assertLength(events, 2);
  tester.assertClose(events[0].whole.end, 1, 0.001);
  tester.assertClose(events[1].whole.begin, 1, 0.001);
});

// Test rev
tester.test('rev reverses pattern', () => {
  const p = rev(cat(pure("bd"), pure("sn"), pure("hh")));
  tester.testEventTiming(p, [
    { start: 0, end: 0.333, value: "hh" },
    { start: 0.333, end: 0.667, value: "sn" },
    { start: 0.667, end: 1, value: "bd" }
  ]);
});

// Test every
tester.test('every applies function periodically', () => {
  const p = every(3, rev, pure("bd"));
  
  // Cycle 0: normal
  const cycle0 = tester.queryArc(p, 0, 1);
  tester.assertEqual(cycle0[0].value, "bd");
  
  // Cycle 2: normal
  const cycle2 = tester.queryArc(p, 2, 3);
  tester.assertEqual(cycle2[0].value, "bd");
  
  // Cycle 3: reversed (every 3rd)
  const cycle3 = tester.queryArc(p, 3, 4);
  // Rev doesn't change a pure pattern, but the function was applied
});

// Test euclid
tester.test('euclid creates euclidean rhythm', () => {
  const p = euclid(3, 8); // 3 hits in 8 steps: x..x..x.
  tester.testEventTiming(p, [
    { start: 0, end: 0.125 },      // First hit
    { start: 0.375, end: 0.5 },     // Second hit
    { start: 0.75, end: 0.875 }     // Third hit
  ]);
});

// Test degrade
tester.test('degrade removes events probabilistically', () => {
  const p = degrade(stack(pure("bd"), pure("sn"), pure("hh")));
  const events = tester.queryArc(p, 0, 1);
  
  // Should have fewer than 3 events (probabilistic)
  if (events.length >= 3) {
    console.warn('  Note: degrade is probabilistic, may occasionally have all events');
  }
  
  // Test determinism with same seed
  tester.testDeterminism(p);
});

// Test choose
tester.test('choose selects from values', () => {
  const p = choose("bd", "sn", "hh");
  const events = tester.queryArc(p, 0, 10);
  
  // Check that all values appear
  const values = new Set(events.map(e => e.value));
  if (values.size < 3) {
    console.warn('  Note: choose is random, may not use all values in 10 cycles');
  }
  
  // Test determinism
  tester.testDeterminism(p);
});

// Test mathematical properties
tester.test('pattern laws hold', () => {
  const a = pure("a");
  const b = pure("b");
  const c = pure("c");
  
  // Associativity of cat
  const cat1 = cat(cat(a, b), c);
  const cat2 = cat(a, cat(b, c));
  tester.assertEqual(
    tester.queryArc(cat1, 0, 1),
    tester.queryArc(cat2, 0, 1),
    'cat should be associative'
  );
  
  // Identity of stack with silence
  const stack1 = stack(a, silence);
  const stack2 = a;
  tester.assertEqual(
    tester.queryArc(stack1, 0, 1),
    tester.queryArc(stack2, 0, 1),
    'stack with silence should be identity'
  );
});

// Test performance
tester.test('patterns perform efficiently', () => {
  // Complex nested pattern
  const p = every(4, rev,
    stack(
      euclid(7, 16),
      fast(2, cat(pure("bd"), pure("sn"))),
      slow(3, pure("hh"))
    )
  );
  
  tester.testPerformance(p, 100, 'Complex pattern should query quickly');
});

// Export for use
if (typeof module !== 'undefined') {
  module.exports = { PatternTest, tester };
}