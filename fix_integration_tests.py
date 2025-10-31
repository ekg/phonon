#!/usr/bin/env python3
"""
Comprehensive Integration Test Fixer

Fixes common issues in integration test files:
1. Duplicate field specifications (pending_freq, last_sample, envelope_type)
2. Missing envelope_type field in SignalNode::Sample
3. Missing pending_freq and last_sample fields in SignalNode::Oscillator

This script is safe to run multiple times (idempotent).
"""

import re
import glob
from typing import List, Tuple

def fix_duplicate_fields(content: str) -> Tuple[str, int]:
    """Remove duplicate field specifications."""
    fixes = 0

    # Pattern: field: value, followed by same field again (with optional whitespace/newlines)
    patterns = [
        (r'(\bpending_freq:\s*None,)\s*\n?\s*pending_freq:\s*None,', r'\1'),
        (r'(\blast_sample:\s*0\.0,)\s*\n?\s*last_sample:\s*0\.0,', r'\1'),
        (r'(\benvelope_type:\s*None,)\s*\n?\s*envelope_type:\s*None,', r'\1'),
    ]

    for pattern, replacement in patterns:
        new_content, count = re.subn(pattern, replacement, content, flags=re.MULTILINE)
        fixes += count
        content = new_content

    return content, fixes

def fix_missing_envelope_type(content: str) -> Tuple[str, int]:
    """Add missing envelope_type field to SignalNode::Sample."""
    fixes = 0

    # Find all SignalNode::Sample { ... } blocks
    def add_envelope_if_missing(match):
        nonlocal fixes
        block = match.group(0)

        # Skip if already has envelope_type
        if 'envelope_type:' in block:
            return block

        # Find the release field and add envelope_type after it
        lines = block.split('\n')
        result = []

        for i, line in enumerate(lines):
            result.append(line)

            # After release field, add envelope_type
            if 'release: Signal::Value' in line and i < len(lines) - 1:
                # Get indentation from current line
                indent = len(line) - len(line.lstrip())
                result.append(' ' * indent + 'envelope_type: None,')
                fixes += 1

        return '\n'.join(result)

    # Match SignalNode::Sample blocks (non-greedy, handles nesting)
    pattern = r'SignalNode::Sample\s*\{[^}]*\}'
    content = re.sub(pattern, add_envelope_if_missing, content, flags=re.DOTALL)

    return content, fixes

def fix_missing_oscillator_fields(content: str) -> Tuple[str, int]:
    """Add missing pending_freq and last_sample to SignalNode::Oscillator."""
    fixes = 0

    def add_oscillator_fields_if_missing(match):
        nonlocal fixes
        block = match.group(0)

        has_pending_freq = 'pending_freq:' in block
        has_last_sample = 'last_sample:' in block

        if has_pending_freq and has_last_sample:
            return block

        lines = block.split('\n')
        result = []

        for i, line in enumerate(lines):
            result.append(line)

            # After phase field, add missing fields
            if 'phase:' in line and i < len(lines) - 1:
                indent = len(line) - len(line.lstrip())

                if not has_pending_freq:
                    result.append(' ' * indent + 'pending_freq: None,')
                    fixes += 1

                if not has_last_sample:
                    result.append(' ' * indent + 'last_sample: 0.0,')
                    fixes += 1

        return '\n'.join(result)

    pattern = r'SignalNode::Oscillator\s*\{[^}]*\}'
    content = re.sub(pattern, add_oscillator_fields_if_missing, content, flags=re.DOTALL)

    return content, fixes

def fix_file(filepath: str) -> Tuple[bool, dict]:
    """Fix a single test file. Returns (changed, stats)."""
    try:
        with open(filepath, 'r') as f:
            original = f.read()

        content = original
        stats = {'duplicates': 0, 'missing_envelope': 0, 'missing_oscillator': 0}

        # Apply fixes in order
        content, stats['duplicates'] = fix_duplicate_fields(content)
        content, stats['missing_envelope'] = fix_missing_envelope_type(content)
        content, stats['missing_oscillator'] = fix_missing_oscillator_fields(content)

        # Only write if changed
        if content != original:
            with open(filepath, 'w') as f:
                f.write(content)
            return True, stats

        return False, stats

    except Exception as e:
        print(f"ERROR processing {filepath}: {e}")
        return False, {}

def main():
    """Fix all integration test files."""
    print("🔧 Comprehensive Integration Test Fixer")
    print("=" * 60)

    # Get all test files
    test_files = glob.glob('tests/test_*.rs') + glob.glob('tests/audio*.rs')
    test_files.sort()

    total_fixed = 0
    total_stats = {'duplicates': 0, 'missing_envelope': 0, 'missing_oscillator': 0}

    for filepath in test_files:
        changed, stats = fix_file(filepath)

        if changed:
            total_fixed += 1
            for key in total_stats:
                total_stats[key] += stats.get(key, 0)

            fixes_list = []
            if stats.get('duplicates'):
                fixes_list.append(f"{stats['duplicates']} duplicates")
            if stats.get('missing_envelope'):
                fixes_list.append(f"{stats['missing_envelope']} envelope_type")
            if stats.get('missing_oscillator'):
                fixes_list.append(f"{stats['missing_oscillator']} oscillator fields")

            print(f"✓ {filepath}: {', '.join(fixes_list)}")

    print("=" * 60)
    print(f"📊 Summary:")
    print(f"   Files fixed: {total_fixed}/{len(test_files)}")
    print(f"   Duplicate fields removed: {total_stats['duplicates']}")
    print(f"   envelope_type fields added: {total_stats['missing_envelope']}")
    print(f"   Oscillator fields added: {total_stats['missing_oscillator']}")
    print()

    if total_fixed > 0:
        print("✅ All integration test files have been fixed!")
    else:
        print("✅ All integration test files are already correct!")

if __name__ == '__main__':
    main()
