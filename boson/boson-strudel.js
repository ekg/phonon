#!/usr/bin/env node

/**
 * Boson with actual Strudel mini notation
 * This properly uses @strudel/mini for parsing
 */

const OSC = require('osc-js');
const fs = require('fs');

// Import actual Strudel packages
// npm install @strudel/mini @strudel/core
const { mini } = require('@strudel/mini');
const { Pattern, stack, sequence } = require('@strudel/core');

class BosonStrudel {
    constructor(config = {}) {
        this.config = {
            oscPort: config.oscPort || 57120,
            oscHost: config.oscHost || 'localhost',
            tempo: config.tempo || 120,
            patternFile: config.patternFile || '../patterns.phonon',
            ...config
        };
        
        this.osc = new OSC({ plugin: new OSC.DatagramPlugin() });
        this.pattern = null;
        this.playing = false;
        
        console.log('🎼 Boson Pattern Engine (Strudel-powered)');
        console.log(`   OSC: ${this.config.oscHost}:${this.config.oscPort}`);
        console.log(`   Tempo: ${this.config.tempo} BPM`);
    }
    
    async init() {
        await this.osc.open({ port: this.config.oscPort + 1 });
        console.log('✓ OSC ready');
        this.loadPattern();
    }
    
    loadPattern() {
        try {
            const content = fs.readFileSync(this.config.patternFile, 'utf8');
            
            // Remove comments and get pattern
            const lines = content.split('\n').filter(l => !l.trim().startsWith('//'));
            const patternLine = lines.find(l => l.trim().length > 0);
            
            if (patternLine) {
                // Use actual Strudel mini notation parser
                // The mini function parses mini notation strings
                this.pattern = mini(patternLine);
                console.log(`✓ Pattern loaded using Strudel mini notation`);
                return true;
            }
            
            // Default pattern - proper house beat
            this.pattern = stack(
                mini("bd*4"),           // Four on floor
                mini("~ cp ~ cp"),      // Clap on 2 and 4
                mini("hh*8")           // Constant hi-hats
            );
            console.log('✓ Loaded default house pattern');
            return true;
            
        } catch (err) {
            console.error('✗ Failed to load pattern:', err.message);
            return false;
        }
    }
    
    play() {
        if (!this.pattern) {
            console.log('No pattern loaded');
            return;
        }
        
        this.playing = true;
        const cps = this.config.tempo / 60 / 4; // cycles per second
        
        console.log('▶ Playing pattern');
        
        // Query the pattern for events
        let cycle = 0;
        
        const tick = () => {
            if (!this.playing) return;
            
            const start = cycle;
            const end = cycle + 1/16; // 16th note resolution
            
            // Query pattern for events in this time slice
            const events = this.pattern.queryArc(start, end);
            
            for (const event of events) {
                this.sendOSC(event);
            }
            
            cycle += 1/16;
            
            // Schedule next tick
            const msPerCycle = 1000 / cps;
            const msPerTick = msPerCycle / 16;
            setTimeout(tick, msPerTick);
        };
        
        tick();
    }
    
    sendOSC(event) {
        const value = event.value;
        
        // Determine type and send appropriate OSC message
        if (typeof value === 'string') {
            // It's a sample
            const [sample, indexStr] = value.split(':');
            const index = parseInt(indexStr) || 0;
            
            const message = new OSC.Message('/sample', sample, index, 1.0);
            this.osc.send(message, { 
                port: this.config.oscPort, 
                host: this.config.oscHost 
            });
            console.log(`  ♫ ${value}`);
            
        } else if (typeof value === 'number') {
            // It's a frequency
            const message = new OSC.Message('/play', value, 0.2);
            this.osc.send(message, { 
                port: this.config.oscPort, 
                host: this.config.oscHost 
            });
            console.log(`  ♪ ${value} Hz`);
            
        } else if (value && value.note) {
            // It's a note object
            const freq = this.noteToFreq(value.note);
            const message = new OSC.Message('/play', freq, 0.2);
            this.osc.send(message, { 
                port: this.config.oscPort, 
                host: this.config.oscHost 
            });
            console.log(`  ♪ ${value.note}`);
        }
    }
    
    noteToFreq(note) {
        // MIDI note number to frequency
        if (typeof note === 'number') {
            return 440 * Math.pow(2, (note - 69) / 12);
        }
        
        // Note name to frequency
        const notes = {
            'c': 261.63, 'd': 293.66, 'e': 329.63, 'f': 349.23,
            'g': 392.00, 'a': 440.00, 'b': 493.88
        };
        
        const match = note.match(/([a-g])([0-9])?/i);
        if (match) {
            const [, noteName, octave] = match;
            const baseFreq = notes[noteName.toLowerCase()];
            const oct = parseInt(octave) || 4;
            return baseFreq * Math.pow(2, oct - 4);
        }
        
        return 440; // Default to A4
    }
    
    stop() {
        this.playing = false;
        console.log('■ Stopped');
    }
    
    watch() {
        console.log(`👁 Watching ${this.config.patternFile}`);
        
        fs.watchFile(this.config.patternFile, { interval: 100 }, (curr, prev) => {
            if (curr.mtime !== prev.mtime) {
                console.log('\n📝 Pattern file changed');
                if (this.loadPattern()) {
                    if (this.playing) {
                        this.stop();
                        this.play();
                    }
                }
            }
        });
    }
}

// CLI
async function main() {
    const args = process.argv.slice(2);
    const command = args[0] || 'play';
    
    const boson = new BosonStrudel();
    await boson.init();
    
    if (command === 'watch') {
        boson.watch();
        boson.play();
        
        process.stdin.resume();
        process.on('SIGINT', () => {
            console.log('\n👋 Shutting down...');
            boson.stop();
            process.exit();
        });
    } else {
        boson.play();
        setTimeout(() => {
            boson.stop();
            process.exit();
        }, 10000);
    }
}

if (require.main === module) {
    main().catch(console.error);
}

module.exports = BosonStrudel;