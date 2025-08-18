#!/usr/bin/env node

/**
 * Boson - Pattern engine for Phonon Forge
 * Powered by Strudel/TidalCycles patterns
 */

const { sequence, stack, Pattern } = require('@strudel/core');
const OSC = require('osc-js');
const fs = require('fs');
const path = require('path');

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
        this.pattern = null;
        this.playing = false;
        this.startTime = Date.now() / 1000;
        this.currentCycle = 0;
        
        console.log('🎼 Boson Pattern Engine');
        console.log(`   OSC: ${this.config.oscHost}:${this.config.oscPort}`);
        console.log(`   Tempo: ${this.config.tempo} BPM`);
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
                const defaultPattern = `// Phonon Forge Pattern File
// Edit and save to hear changes!

// Simple melody
"440 550 660 550"

// Or use Strudel syntax:
// sequence("c3", "e3", "g3", "c4")
`;
                fs.writeFileSync(this.config.patternFile, defaultPattern);
                console.log(`✓ Created ${this.config.patternFile}`);
            }
            
            const content = fs.readFileSync(this.config.patternFile, 'utf8');
            
            // Extract pattern (simple parser)
            const lines = content.split('\n').filter(l => !l.trim().startsWith('//'));
            const patternLine = lines.find(l => l.includes('"'));
            
            if (patternLine) {
                const match = patternLine.match(/"([^"]+)"/);
                if (match) {
                    const notes = match[1].split(/\s+/);
                    this.pattern = notes.map(n => {
                        if (n === '~') return null;
                        return parseFloat(n) || this.noteToFreq(n);
                    });
                    console.log(`✓ Pattern loaded: ${this.pattern.filter(p => p).join(' ')}`);
                    return true;
                }
            }
            
            console.log('⚠ No pattern found, using default');
            this.pattern = [440, 550, 660, 550];
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
        console.log('■ Stopped');
    }
    
    scheduleNextBeat(index) {
        if (!this.playing) return;
        
        const note = this.pattern[index % this.pattern.length];
        
        if (note) {
            // Send OSC to Fermion
            const message = new OSC.Message('/play', note, 0.2);
            this.osc.send(message, { 
                port: this.config.oscPort, 
                host: this.config.oscHost 
            });
            
            console.log(`  ♪ ${note} Hz`);
        }
        
        // Schedule next beat
        const beatDuration = (60 / this.config.tempo) * 1000 / 4;
        setTimeout(() => {
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
    
    const boson = new Boson();
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