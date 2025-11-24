#!/usr/bin/env python3
import re
import sys

def fix_oscillator_patterns(content):
    """Fix SignalNode::Oscillator pattern matches to include semitone_offset"""

    # Pattern for matching Oscillator destructuring
    pattern = r'(SignalNode::Oscillator\s*\{\s*)'
    pattern += r'((?:freq|waveform|phase|pending_freq|last_sample)\s*[:,].*?)*'
    pattern += r'(\})'

    # Find all Oscillator patterns and add semitone_offset if missing
    def replace_pattern(match):
        start = match.group(1)
        middle = match.group(2) if match.group(2) else ""
        end = match.group(3)

        # Check if semitone_offset is already present
        if 'semitone_offset' in middle:
            return match.group(0)

        # Add semitone_offset: _
        if middle:
            return f"{start}{middle}\n            semitone_offset: _,{end}"
        else:
            return f"{start}semitone_offset: _,{end}"

    return re.sub(pattern, replace_pattern, content, flags=re.MULTILINE | re.DOTALL)

if __name__ == "__main__":
    for filename in sys.argv[1:]:
        with open(filename, 'r') as f:
            content = f.read()

        new_content = fix_oscillator_patterns(content)

        if new_content != content:
            with open(filename, 'w') as f:
                f.write(new_content)
            print(f"Fixed {filename}")
