/**
 * Pattern DSL Parser for Phonon
 * Implements a subset of Strudel/TidalCycles syntax
 */

class PatternParser {
    constructor() {
        // Note to frequency mapping
        this.notes = {
            'c0': 16.35, 'c#0': 17.32, 'd0': 18.35, 'd#0': 19.45, 'e0': 20.60, 'f0': 21.83,
            'f#0': 23.12, 'g0': 24.50, 'g#0': 25.96, 'a0': 27.50, 'a#0': 29.14, 'b0': 30.87,
            'c1': 32.70, 'c#1': 34.65, 'd1': 36.71, 'd#1': 38.89, 'e1': 41.20, 'f1': 43.65,
            'f#1': 46.25, 'g1': 49.00, 'g#1': 51.91, 'a1': 55.00, 'a#1': 58.27, 'b1': 61.74,
            'c2': 65.41, 'c#2': 69.30, 'd2': 73.42, 'd#2': 77.78, 'e2': 82.41, 'f2': 87.31,
            'f#2': 92.50, 'g2': 98.00, 'g#2': 103.83, 'a2': 110.00, 'a#2': 116.54, 'b2': 123.47,
            'c3': 130.81, 'c#3': 138.59, 'd3': 146.83, 'd#3': 155.56, 'e3': 164.81, 'f3': 174.61,
            'f#3': 185.00, 'g3': 196.00, 'g#3': 207.65, 'a3': 220.00, 'a#3': 233.08, 'b3': 246.94,
            'c4': 261.63, 'c#4': 277.18, 'd4': 293.66, 'd#4': 311.13, 'e4': 329.63, 'f4': 349.23,
            'f#4': 369.99, 'g4': 392.00, 'g#4': 415.30, 'a4': 440.00, 'a#4': 466.16, 'b4': 493.88,
            'c5': 523.25, 'c#5': 554.37, 'd5': 587.33, 'd#5': 622.25, 'e5': 659.25, 'f5': 698.46,
            'f#5': 739.99, 'g5': 783.99, 'g#5': 830.61, 'a5': 880.00, 'a#5': 932.33, 'b5': 987.77,
        };
        
        // Strudel sample mapping
        // Format: "sample:index" where index selects which file
        // e.g., "bd" = bd:0, "bd:1" = second bd sample
        this.samples = {
            // Kicks
            'bd': 'bd', 'kick': 'bd',
            // Snares  
            'sn': 'sn', 'sd': 'sn', 'snare': 'sn',
            // Hi-hats
            'hh': 'hh', 'hihat': 'hh', 'hat': 'hh',
            'oh': 'oh', 'openhat': 'oh', 'openhihat': 'oh',
            // Claps
            'cp': 'cp', 'clap': 'cp',
            // Cymbals
            'cr': 'cr', 'crash': 'cr',
            'cy': 'cr', 'cymbal': 'cr',
            // Rimshot
            'rs': 'rs', 'rim': 'rs', 'rimshot': 'rs',
            // Cowbell
            'cb': 'cb', 'cowbell': 'cb',
            // Bass
            'bass': 'bass', 'bs': 'bass',
            // Toms
            'lt': 'lt', 'lowtom': 'lt',
            'mt': 'mt', 'midtom': 'mt', 
            'ht': 'ht', 'hightom': 'ht',
            // Percussion
            'perc': 'perc',
            // Effects
            'fx': 'fx',
            // Vocals
            'voc': 'voc', 'vocal': 'voc',
        };
    }
    
    /**
     * Parse a pattern string into events
     * Supports:
     * - "440 550" - frequencies
     * - "c4 e4 g4" - note names
     * - "bd sd hh" - samples
     * - "[c4,e4,g4]" - chords
     * - "c4*2" - repeat
     * - "~" - rest
     * - "c4:2" - with duration
     * - "bd*4, ~ cp ~ cp" - stacked patterns (comma-separated)
     */
    parse(pattern) {
        if (!pattern || typeof pattern !== 'string') {
            return [];
        }
        
        // Remove comments
        pattern = pattern.replace(/\/\/.*$/gm, '').trim();
        
        // Check for quoted string pattern
        const quotedMatch = pattern.match(/"([^"]+)"/);
        if (quotedMatch) {
            pattern = quotedMatch[1];
        }
        
        // Check for stacked patterns (comma-separated)
        if (pattern.includes(',') && !pattern.includes('[')) {
            // This is a stack - parse each layer
            const layers = pattern.split(',').map(p => p.trim());
            return this.parseStack(layers);
        }
        
        // Parse single pattern
        const events = [];
        const tokens = this.tokenize(pattern);
        
        for (const token of tokens) {
            const event = this.parseToken(token);
            if (event) {
                events.push(event);
            }
        }
        
        return events;
    }
    
    tokenize(pattern) {
        // Split by spaces but respect brackets
        const tokens = [];
        let current = '';
        let depth = 0;
        
        for (let i = 0; i < pattern.length; i++) {
            const char = pattern[i];
            
            if (char === '[') {
                depth++;
                current += char;
            } else if (char === ']') {
                depth--;
                current += char;
            } else if (char === ' ' && depth === 0) {
                if (current) {
                    tokens.push(current);
                    current = '';
                }
            } else {
                current += char;
            }
        }
        
        if (current) {
            tokens.push(current);
        }
        
        return tokens;
    }
    
    parseToken(token) {
        // Check for rest
        if (token === '~' || token === '.') {
            return { type: 'rest' };
        }
        
        // Check for chord [c4,e4,g4]
        if (token.startsWith('[') && token.endsWith(']')) {
            const inner = token.slice(1, -1);
            const notes = inner.split(',').map(n => this.parseToken(n.trim()));
            return { type: 'chord', notes: notes.filter(n => n) };
        }
        
        // Check for repeat c4*3
        if (token.includes('*')) {
            const [base, count] = token.split('*');
            const event = this.parseToken(base);
            if (event) {
                event.repeat = parseInt(count) || 1;
            }
            return event;
        }
        
        // Check for duration c4:0.5
        let duration = null;
        if (token.includes(':')) {
            const [base, dur] = token.split(':');
            token = base;
            duration = parseFloat(dur);
        }
        
        // Check if it's a number (frequency)
        const freq = parseFloat(token);
        if (!isNaN(freq)) {
            return { type: 'freq', value: freq, duration };
        }
        
        // Check if it's a note name
        const noteLower = token.toLowerCase();
        if (this.notes[noteLower]) {
            return { type: 'note', value: this.notes[noteLower], name: token, duration };
        }
        
        // Check if it's a sample name
        if (this.samples[noteLower]) {
            return { type: 'sample', value: this.samples[noteLower], name: token, duration };
        }
        
        // Check for Strudel sample syntax: "sample:index" or "sample"
        const sampleMatch = token.match(/^([a-z]+)(:(\d+))?$/i);
        if (sampleMatch) {
            const [, name, , index] = sampleMatch;
            const sampleName = name.toLowerCase();
            
            // Check if it's a known sample
            if (this.samples[sampleName]) {
                return { 
                    type: 'sample', 
                    value: this.samples[sampleName], 
                    name: token,
                    index: index ? parseInt(index) : 0,
                    duration 
                };
            }
            
            // Unknown samples are treated as custom samples
            return { 
                type: 'sample', 
                value: sampleName, 
                name: token,
                index: index ? parseInt(index) : 0,
                duration 
            };
        }
        
        // Default to treating as sample
        return { type: 'sample', value: token, name: token, duration };
    }
    
    /**
     * Parse stacked patterns (comma-separated layers)
     * Each layer plays simultaneously
     */
    parseStack(layers) {
        const allEvents = [];
        
        // Parse each layer
        const parsedLayers = layers.map(layer => {
            const tokens = this.tokenize(layer);
            const events = [];
            for (const token of tokens) {
                const event = this.parseToken(token);
                if (event) {
                    events.push(event);
                }
            }
            return this.expand(events);
        });
        
        // Find the longest pattern
        const maxLength = Math.max(...parsedLayers.map(l => l.length));
        
        // Merge all layers into simultaneous events
        for (let i = 0; i < maxLength; i++) {
            const simultaneousEvents = [];
            
            for (const layer of parsedLayers) {
                if (layer.length > 0) {
                    const event = layer[i % layer.length];
                    if (event && event.type !== 'rest') {
                        simultaneousEvents.push(event);
                    }
                }
            }
            
            if (simultaneousEvents.length > 0) {
                if (simultaneousEvents.length === 1) {
                    allEvents.push(simultaneousEvents[0]);
                } else {
                    // Multiple simultaneous events
                    allEvents.push({
                        type: 'stack',
                        events: simultaneousEvents
                    });
                }
            } else {
                allEvents.push({ type: 'rest' });
            }
        }
        
        return allEvents;
    }
    
    /**
     * Expand pattern with repeats and convert to flat event list
     */
    expand(events) {
        const expanded = [];
        
        for (const event of events) {
            if (event.type === 'chord') {
                // For chords, play all notes at once
                expanded.push(event);
            } else if (event.repeat) {
                // Repeat the event
                for (let i = 0; i < event.repeat; i++) {
                    expanded.push({ ...event, repeat: undefined });
                }
            } else {
                expanded.push(event);
            }
        }
        
        return expanded;
    }
}

module.exports = PatternParser;