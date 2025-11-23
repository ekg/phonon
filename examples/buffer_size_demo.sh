#!/bin/bash
# Buffer Size Configuration Demo
# Demonstrates different buffer sizes and their latency impact

echo "================================================"
echo "Phonon Buffer Size Configuration Demo"
echo "================================================"
echo ""
echo "This demo shows how to configure Phonon's audio buffer size"
echo "to tune latency vs CPU usage for your needs."
echo ""
echo "Press Ctrl+C to stop each test and move to the next."
echo ""
echo "================================================"
echo ""

# Function to run with specific buffer size
run_test() {
    local size=$1
    local latency=$2
    local description=$3

    echo "--------------------------------------------"
    echo "Test $test_num: Buffer Size = $size samples ($latency)"
    echo "Description: $description"
    echo "--------------------------------------------"
    echo ""
    echo "Starting Phonon with PHONON_BUFFER_SIZE=$size..."
    echo "Watch for the '🔧 Buffer size' line in the output below."
    echo ""

    PHONON_BUFFER_SIZE=$size cargo run --release --bin phonon -- live examples/simple_working_beat.ph 2>&1 | head -20

    echo ""
    echo "Test $test_num complete."
    echo ""
    ((test_num++))
}

test_num=1

# Test 1: Ultra-low latency
run_test 64 "1.5ms" "Ultra-low latency - Best for live performance with powerful CPU"

# Test 2: Default (low latency)
run_test 128 "3ms" "Low latency - Recommended default, good balance"

# Test 3: Medium latency
run_test 256 "6ms" "Medium latency - Better for complex FX chains"

# Test 4: High latency
run_test 512 "12ms" "High latency - Maximum CPU headroom"

echo "================================================"
echo "Demo Complete!"
echo "================================================"
echo ""
echo "Key Takeaways:"
echo "1. Smaller buffers = lower latency but more CPU usage"
echo "2. Larger buffers = higher latency but more stable"
echo "3. Default (128 samples, 3ms) is a good starting point"
echo "4. Adjust based on your CPU and patch complexity"
echo ""
echo "To use in your own sessions:"
echo "  PHONON_BUFFER_SIZE=128 phonon live your_code.ph"
echo ""
