#!/bin/bash
# Extract parameter names from all compile_* functions in compositional_compiler.rs

echo "# Parameter Extraction from compile_* functions"
echo ""

# Get list of all compile_* function names (excluding compile_expr, compile_statement, etc)
functions=$(grep -oP '^fn compile_\K[a-z_]+(?=\(ctx:)' /home/erik/phonon/src/compositional_compiler.rs | grep -v '^expr$\|^statement$\|^constant\|^unipolar\|^bipolar\|^user_function' | sort -u)

for func in $functions; do
    # Extract the function definition and the following 50 lines to get param extractions
    params=$(awk "/^fn compile_${func}\(/,/^fn compile_|^pub fn compile_|^}\$/" /home/erik/phonon/src/compositional_compiler.rs | \
        grep -oP 'get_required\(\d+, "([^"]+)"\)|get_optional\(\d+, "([^"]+)"' | \
        grep -oP '"([^"]+)"' | tr -d '"' | sort -u)

    if [ -n "$params" ]; then
        echo "## $func"
        echo "$params"
        echo ""
    fi
done
