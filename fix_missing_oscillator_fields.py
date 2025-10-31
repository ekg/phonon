#!/usr/bin/env python3
"""
Fix missing oscillator fields in example files.

Ensures all SignalNode::Oscillator nodes have:
- pending_freq: None,
- last_sample: 0.0,
"""

import re
import sys
from pathlib import Path

def fix_oscillator_fields(content: str) -> tuple[str, int]:
    """Add missing oscillator fields."""
    lines = content.split('\n')
    result = []
    fixes = 0

    i = 0
    while i < len(lines):
        line = lines[i]
        result.append(line)

        # Check if this line starts an Oscillator node
        if 'SignalNode::Oscillator {' in line:
            # Look ahead to find the closing brace
            struct_lines = [line]
            i += 1
            brace_count = line.count('{') - line.count('}')

            while i < len(lines) and brace_count > 0:
                struct_lines.append(lines[i])
                brace_count += lines[i].count('{') - lines[i].count('}')
                i += 1

            # Check if pending_freq and last_sample are present
            struct_text = '\n'.join(struct_lines)
            has_pending_freq = 'pending_freq:' in struct_text
            has_last_sample = 'last_sample:' in struct_text

            if not has_pending_freq or not has_last_sample:
                # Find the closing brace line
                closing_line_idx = len(struct_lines) - 1
                while closing_line_idx >= 0:
                    if '});' in struct_lines[closing_line_idx] or '})' in struct_lines[closing_line_idx]:
                        break
                    closing_line_idx -= 1

                # Get indentation from previous line
                prev_line = struct_lines[closing_line_idx - 1] if closing_line_idx > 0 else struct_lines[0]
                indent = len(prev_line) - len(prev_line.lstrip())
                indent_str = ' ' * indent

                # Insert missing fields before closing brace
                insert_lines = []
                if not has_pending_freq:
                    insert_lines.append(f'{indent_str}pending_freq: None,')
                    fixes += 1
                if not has_last_sample:
                    insert_lines.append(f'{indent_str}last_sample: 0.0,')
                    fixes += 1

                # Add to result (skip the first line as it's already added)
                for j in range(1, closing_line_idx):
                    result.append(struct_lines[j])

                # Add the missing fields
                for insert_line in insert_lines:
                    result.append(insert_line)

                # Add the closing brace
                result.append(struct_lines[closing_line_idx])
            else:
                # Add all lines as-is (skip first as already added)
                for j in range(1, len(struct_lines)):
                    result.append(struct_lines[j])

            continue

        i += 1

    return '\n'.join(result), fixes

def process_file(filepath: Path) -> bool:
    """Process a single file."""
    try:
        content = filepath.read_text()
        fixed_content, num_fixes = fix_oscillator_fields(content)

        if num_fixes > 0:
            filepath.write_text(fixed_content)
            print(f"✅ {filepath}: Added {num_fixes} missing oscillator fields")
            return True
        else:
            print(f"✓  {filepath}: No fixes needed")
            return False
    except Exception as e:
        print(f"❌ {filepath}: Error - {e}")
        return False

def main():
    example_files = [
        Path('examples/live.rs'),
        Path('examples/phonon_poll.rs'),
        Path('examples/phonon_live.rs'),
        Path('examples/live_playground.rs'),
    ]

    total_fixed = 0
    for filepath in example_files:
        if filepath.exists():
            if process_file(filepath):
                total_fixed += 1

    print(f"\n✅ Fixed {total_fixed} files")

if __name__ == '__main__':
    main()
