#!/usr/bin/env python3
"""
Fix Oscillator, FMOscillator, and PMOscillator field initialization to use RefCell.
"""

import sys
import re
from pathlib import Path

def fix_oscillator_fields(content):
    """Replace bare field initializers with RefCell::new() wrapped versions."""

    # Fix regular Oscillator phase fields
    content = re.sub(
        r'(\s+)phase:\s*(\d+\.\d+|[^,\n]+),\s*$',
        r'\1phase: RefCell::new(\2),',
        content,
        flags=re.MULTILINE
    )

    # Fix pending_freq fields
    content = re.sub(
        r'(\s+)pending_freq:\s*None,\s*$',
        r'\1pending_freq: RefCell::new(None),',
        content,
        flags=re.MULTILINE
    )

    # Fix last_sample fields
    content = re.sub(
        r'(\s+)last_sample:\s*(\d+\.\d+),\s*$',
        r'\1last_sample: RefCell::new(\2),',
        content,
        flags=re.MULTILINE
    )

    # Fix FMOscillator carrier_phase and modulator_phase
    content = re.sub(
        r'(\s+)carrier_phase:\s*(\d+\.\d+),\s*$',
        r'\1carrier_phase: RefCell::new(\2),',
        content,
        flags=re.MULTILINE
    )

    content = re.sub(
        r'(\s+)modulator_phase:\s*(\d+\.\d+),\s*$',
        r'\1modulator_phase: RefCell::new(\2),',
        content,
        flags=re.MULTILINE
    )

    return content

def add_refcell_import(content):
    """Add RefCell import if not present and needed."""

    # Check if we have Oscillator constructions
    has_oscillator = 'SignalNode::Oscillator' in content or \
                     'SignalNode::FMOscillator' in content or \
                     'SignalNode::PMOscillator' in content

    # Check if RefCell is already imported
    has_refcell_import = re.search(r'use std::cell::RefCell', content)

    if has_oscillator and not has_refcell_import:
        # Find the first 'use' statement location
        use_match = re.search(r'^use\s+', content, re.MULTILINE)
        if use_match:
            # Insert before first use
            pos = use_match.start()
            content = content[:pos] + 'use std::cell::RefCell;\n' + content[pos:]
        else:
            # No use statements, add after any #! or // at the top
            lines = content.split('\n')
            insert_pos = 0
            for i, line in enumerate(lines):
                if not line.startswith('#!') and not line.startswith('//') and line.strip():
                    insert_pos = i
                    break
            lines.insert(insert_pos, 'use std::cell::RefCell;')
            content = '\n'.join(lines)

    return content

def process_file(filepath):
    """Process a single file."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original = content

        # Fix oscillator fields
        content = fix_oscillator_fields(content)

        # Add import if needed
        content = add_refcell_import(content)

        # Only write if changed
        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return False

def main():
    """Process all Rust files."""

    if len(sys.argv) > 1:
        # Process specific files
        files = [Path(f) for f in sys.argv[1:]]
    else:
        # Process all .rs files in src, tests, examples
        base = Path(__file__).parent
        files = []
        for pattern in ['src/**/*.rs', 'tests/**/*.rs', 'examples/**/*.rs']:
            files.extend(base.glob(pattern))

    updated = 0
    for filepath in files:
        if process_file(filepath):
            print(f"Updated: {filepath}")
            updated += 1

    print(f"\nTotal files updated: {updated}")

if __name__ == '__main__':
    main()
