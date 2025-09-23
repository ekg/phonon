#!/bin/bash

echo "=== Phonon Test Coverage Summary ==="
echo ""
echo "Test Files:"
echo "-----------"
echo "Total test files: $(find tests -name "*.rs" | wc -l)"
echo "Test modules in lib.rs: $(grep -c "#\[test\]" src/*.rs 2>/dev/null || echo 0)"
echo ""

echo "Test Categories:"
echo "----------------"
find tests -name "*.rs" | xargs basename -s .rs | sed 's/^test_//' | sort | uniq -c | sort -rn | head -10

echo ""
echo "Running tests..."
echo "----------------"
cargo test 2>&1 | tee /tmp/test_output.log | grep "test result:" | tail -1

echo ""
echo "Test Statistics:"
echo "----------------"
PASSED=$(grep -c "test.*ok" /tmp/test_output.log || echo 0)
FAILED=$(grep -c "test.*FAILED" /tmp/test_output.log || echo 0)
echo "Tests passed: $PASSED"
echo "Tests failed: $FAILED"

echo ""
echo "Key Test Areas:"
echo "---------------"
echo "Pattern tests: $(find tests -name "*pattern*.rs" | wc -l)"
echo "Audio tests: $(find tests -name "*audio*.rs" | wc -l)"
echo "DSP tests: $(find tests -name "*dsp*.rs" | wc -l)"
echo "Filter tests: $(find tests -name "*filter*.rs" | wc -l)"
echo "Modulation tests: $(find tests -name "*modulation*.rs" | wc -l)"
echo "E2E tests: $(find tests -name "*e2e*.rs" -o -name "*end_to_end*.rs" | wc -l)"
echo "System tests: $(find tests -name "*system*.rs" | wc -l)"