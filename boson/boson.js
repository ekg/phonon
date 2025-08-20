#!/usr/bin/env node

/**
 * Boson - Pattern engine for Phonon
 * Powered by Strudel/TidalCycles patterns
 */

const { sequence, stack, Pattern } = require('@strudel/core');
const OSC = require('osc-js');
const fs = require('fs');
const path = require('path');
const PatternParser = require('./parser');

class Boson {
    constructor(config = {}) {
        this.config = {
            oscPort: config.oscPort || 57120,
            oscHost: config.oscHost || 'localhost',
            tempo: config.tempo || 120,
            patternFile: config.patternFile || 'patterns.phonon',
            ...config
        };
        
        this.osc = new OSC({ plugin: new OSC.DatagramPlugin() });
        this.parser = new PatternParser();
        this.pattern = null;
        this.playing = false;
        this.startTime = Date.now() / 1000;
        this.currentCycle = 0;
        this.scheduledTimeout = null;  // Track the timeout so we can cancel it
        
        console.log('🎼 Boson Pattern Engine');
        console.log(`   OSC: ${this.config.oscHost}:${this.config.oscPort}`);
        console.log(`   Tempo: ${this.config.tempo} BPM`);
        console.log(`   Pattern file: ${this.config.patternFile}`);
    }
    
    async init() {
        // Open OSC connection
        await this.osc.open({ port: this.config.oscPort + 1 });
        console.log('✓ OSC ready');
        
        // Load initial pattern
        this.loadPattern();
    }
    
    loadPattern() {
        try {
            if (!fs.existsSync(this.config.patternFile)) {
                // Create default pattern file
                const defaultPattern = `// Phonon Pattern File
// Edit and save to hear changes!

// Drum pattern
"bd ~ sd ~"

// Or try:
// "bd bd sd hh*4"  // Kick kick snare hihat×4
// "c4 e4 g4 c5"     // C major arpeggio
// "[c4,e4,g4] ~"    // Chord then rest
// "bd:0.5 sd:0.2"   // With durations
`;
                fs.writeFileSync(this.config.patternFile, defaultPattern);
                console.log(`✓ Created ${this.config.patternFile}`);
            }
            
            const content = fs.readFileSync(this.config.patternFile, 'utf8');
            
            // Parse pattern using the new parser
            const events = this.parser.parse(content);
            
            if (events.length > 0) {
                this.pattern = this.parser.expand(events);
                const summary = this.pattern.map(e => {
                    if (e.type === 'rest') return '~';
                    if (e.type === 'sample') return e.name;
                    if (e.type === 'note') return e.name;
                    if (e.type === 'freq') return `${e.value}Hz`;
                    if (e.type === 'chord') return '[chord]';
                    return '?';
                }).join(' ');
                console.log(`✓ Pattern loaded: ${summary}`);
                return true;
            }
            
            console.log('⚠ No pattern found, using default');
            this.pattern = [
                { type: 'sample', value: 'bd', name: 'bd' },
                { type: 'rest' },
                { type: 'sample', value: 'sd', name: 'sd' },
                { type: 'rest' }
            ];
            return true;
            
        } catch (err) {
            console.error('✗ Failed to load pattern:', err.message);
            return false;
        }
    }
    
    noteToFreq(note) {
        // Convert note names to frequencies
        const notes = {
            'c3': 130.81, 'd3': 146.83, 'e3': 164.81, 'f3': 174.61,
            'g3': 196.00, 'a3': 220.00, 'b3': 246.94,
            'c4': 261.63, 'd4': 293.66, 'e4': 329.63, 'f4': 349.23,
            'g4': 392.00, 'a4': 440.00, 'b4': 493.88, 'c5': 523.25
        };
        return notes[note.toLowerCase()] || 440;
    }
    
    play() {
        if (!this.pattern) {
            console.log('No pattern loaded');
            return;
        }
        
        this.playing = true;
        this.startTime = Date.now() / 1000;
        this.currentCycle = 0;
        
        console.log('▶ Playing pattern');
        this.scheduleNextBeat(0);
    }
    
    stop() {
        this.playing = false;
        if (this.scheduledTimeout) {
            clearTimeout(this.scheduledTimeout);
            this.scheduledTimeout = null;
        }
        console.log('■ Stopped');
    }
    
    playEvent(event, duration) {
        if (event.type === 'sample') {
            const index = event.index || 0;
            const message = new OSC.Message('/sample', event.value, index, 1.0);
            this.osc.send(message, { 
                port: this.config.oscPort, 
                host: this.config.oscHost 
            });
        } else if (event.type === 'note' || event.type === 'freq') {
            const message = new OSC.Message('/play', event.value, duration);
            this.osc.send(message, { 
                port: this.config.oscPort, 
                host: this.config.oscHost 
            });
        }
    }
    
    scheduleNextBeat(index) {
        if (!this.playing) return;
        
        const event = this.pattern[index % this.pattern.length];
        
        if (event && event.type !== 'rest') {
            const duration = event.duration || 0.2;
            
            if (event.type === 'stack') {
                // Handle stacked/simultaneous events
                for (const e of event.events) {
                    this.playEvent(e, duration);
                }
                console.log(`  ♫ [stack]`);
            } else if (event.type === 'sample') {
                // Send sample message with index
                const index = event.index || 0;
                const message = new OSC.Message('/sample', event.value, index, 1.0);
                this.osc.send(message, { 
                    port: this.config.oscPort, 
                    host: this.config.oscHost 
                });
                console.log(`  ♫ ${event.name}`);
            } else if (event.type === 'note' || event.type === 'freq') {
                // Send frequency message
                const message = new OSC.Message('/play', event.value, duration);
                this.osc.send(message, { 
                    port: this.config.oscPort, 
                    host: this.config.oscHost 
                });
                console.log(`  ♪ ${event.name || event.value}`);
            } else if (event.type === 'chord') {
                // Send chord as multiple notes
                for (const note of event.notes) {
                    if (note.type !== 'rest') {
                        const msg = new OSC.Message('/play', note.value, duration);
                        this.osc.send(msg, { 
                            port: this.config.oscPort, 
                            host: this.config.oscHost 
                        });
                    }
                }
                console.log(`  ♫ [chord]`);
            }
        }
        
        // Schedule next beat
        const beatDuration = (60 / this.config.tempo) * 1000 / 4;
        this.scheduledTimeout = setTimeout(() => {
            this.scheduleNextBeat(index + 1);
            
            if ((index + 1) % this.pattern.length === 0) {
                this.currentCycle++;
                if (this.currentCycle % 4 === 0) {
                    console.log(`  [Cycle ${this.currentCycle}]`);
                }
            }
        }, beatDuration);
    }
    
    watch() {
        console.log(`👁 Watching ${this.config.patternFile}`);
        
        fs.watchFile(this.config.patternFile, { interval: 100 }, (curr, prev) => {
            if (curr.mtime !== prev.mtime) {
                console.log('\n📝 Pattern file changed');
                // Just reload the pattern - don't restart the scheduling loop!
                this.loadPattern();
            }
        });
    }
}

// CLI
async function main() {
    const args = process.argv.slice(2);
    const command = args[0] || 'play';
    
    // Check if second argument is a pattern file path
    const config = {};
    if (args[1] && !args[1].startsWith('-')) {
        config.patternFile = args[1];
    }
    
    const boson = new Boson(config);
    await boson.init();
    
    if (command === 'watch') {
        boson.watch();
        boson.play();
        
        // Keep running
        process.stdin.resume();
        process.on('SIGINT', () => {
            console.log('\n👋 Shutting down...');
            boson.stop();
            process.exit();
        });
    } else {
        boson.play();
        
        // Play for 10 seconds then exit
        setTimeout(() => {
            boson.stop();
            process.exit();
        }, 10000);
    }
}

if (require.main === module) {
    main().catch(console.error);
}

module.exports = Boson;