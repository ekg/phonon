#!/usr/bin/env node

/**
 * Boson with REAL Strudel mini-notation parser
 * 100% semantic compatibility
 */

const OSC = require('osc-js');
const fs = require('fs');
const { mini } = require('@strudel/mini');
const { Pattern, stack } = require('@strudel/core');

class RealBoson {
    constructor(config = {}) {
        // Default BPM and calculate cycles per second
        const bpm = config.bpm || 120;
        const beatsPerBar = config.beatsPerBar || 4;
        const cps = config.cps || (bpm / 60 / beatsPerBar); // 120 BPM = 0.5 cps for 4/4 time
        
        this.config = {
            oscPort: config.oscPort || 57120,
            oscHost: config.oscHost || 'localhost',
            bpm: bpm,
            beatsPerBar: beatsPerBar,
            cps: cps,
            patternFile: config.patternFile || '../patterns.phonon',
            ...config
        };
        
        this.osc = new OSC({ plugin: new OSC.DatagramPlugin() });
        this.pattern = null;
        this.playing = false;
        this.startTime = null;
        this.scheduledEvents = new Set();
        
        console.log('🎼 Boson with REAL Strudel mini-notation');
        console.log(`   BPM: ${this.config.bpm} (${this.config.cps.toFixed(3)} cycles/sec)`);
        console.log(`   Pattern: ${this.config.patternFile}`);
    }
    
    async init() {
        await this.osc.open({ port: this.config.oscPort + 1 });
        console.log('✓ OSC ready');
        this.loadPattern();
    }
    
    loadPattern() {
        try {
            const content = fs.readFileSync(this.config.patternFile, 'utf8');
            
            // Extract only uncommented quoted patterns
            const patterns = [];
            const lines = content.split('\n');
            
            for (const line of lines) {
                // Skip lines that start with //
                if (line.trim().startsWith('//')) continue;
                
                // Extract quoted patterns from non-comment lines
                const regex = /"([^"]+)"/g;
                let match;
                while ((match = regex.exec(line)) !== null) {
                    patterns.push(match[1]);
                }
            }
            
            if (patterns.length === 0) {
                console.log('No patterns found - playing silence');
                // Create a silence pattern
                this.pattern = require('@strudel/core').silence;
                return true;
            }
            
            // Parse each pattern with Strudel's mini parser
            const parsedPatterns = patterns.map(p => {
                console.log(`  Parsing: "${p}"`);
                return mini(p);
            });
            
            // Stack if multiple patterns, otherwise use single
            this.pattern = parsedPatterns.length > 1 
                ? stack(...parsedPatterns) 
                : parsedPatterns[0];
                
            console.log(`✓ Loaded ${patterns.length} pattern(s)`);
            return true;
            
        } catch (err) {
            console.error('Failed to load:', err.message);
            return false;
        }
    }
    
    play() {
        if (!this.pattern) {
            console.log('No pattern loaded');
            return;
        }
        
        this.playing = true;
        this.startTime = Date.now() / 1000;
        this.scheduledEvents = new Set();
        
        console.log('▶ Playing');
        this.scheduleEvents();
    }
    
    stop() {
        this.playing = false;
        if (this.timeout) clearTimeout(this.timeout);
        console.log('■ Stopped');
    }
    
    scheduleEvents() {
        if (!this.playing || !this.pattern) return;
        
        const now = (Date.now() / 1000) - this.startTime;
        const lookAhead = 0.1; // 100ms lookahead
        const endTime = now + lookAhead;
        
        // Query pattern for upcoming events
        const events = this.pattern.queryArc(
            now * this.config.cps,
            endTime * this.config.cps
        );
        
        // Track scheduled events to avoid duplicates
        const scheduled = new Set();
        
        // Schedule OSC messages
        for (const event of events) {
            const eventTime = event.whole.begin / this.config.cps;
            const eventKey = `${event.whole.begin}-${event.value}`;
            
            // Skip if already scheduled
            if (this.scheduledEvents && this.scheduledEvents.has(eventKey)) {
                continue;
            }
            
            if (eventTime >= now) {
                const delay = Math.max(0, (eventTime - now) * 1000);
                scheduled.add(eventKey);
                
                setTimeout(() => {
                    if (!this.playing) return;
                    
                    // Handle different value types
                    if (typeof event.value === 'string') {
                        // Sample name
                        const message = new OSC.Message('/sample', event.value, 0, 1.0);
                        this.osc.send(message, {
                            port: this.config.oscPort,
                            host: this.config.oscHost
                        });
                        console.log(`  ♫ ${event.value}`);
                    } else if (typeof event.value === 'number') {
                        // Note/frequency
                        const message = new OSC.Message('/play', event.value, 0.2);
                        this.osc.send(message, {
                            port: this.config.oscPort,
                            host: this.config.oscHost
                        });
                        console.log(`  ♪ ${event.value}Hz`);
                    } else if (Array.isArray(event.value) && event.value.length === 2) {
                        // Sample:index format [sample_name, index]
                        const [sample, index] = event.value;
                        const message = new OSC.Message('/sample', sample, index, 1.0);
                        this.osc.send(message, {
                            port: this.config.oscPort,
                            host: this.config.oscHost
                        });
                        console.log(`  ♫ ${sample}:${index}`);
                    } else if (event.value && event.value.s) {
                        // Strudel sample object
                        const message = new OSC.Message('/sample', event.value.s, event.value.n || 0, 1.0);
                        this.osc.send(message, {
                            port: this.config.oscPort,
                            host: this.config.oscHost
                        });
                        console.log(`  ♫ ${event.value.s}`);
                    }
                }, delay);
            }
        }
        
        // Update scheduled events set
        if (!this.scheduledEvents) this.scheduledEvents = new Set();
        for (const key of scheduled) {
            this.scheduledEvents.add(key);
        }
        
        // Clean up old events from the set (keep only future events)
        for (const key of this.scheduledEvents) {
            const time = parseFloat(key.split('-')[0]);
            if (time / this.config.cps < now - 0.5) {
                this.scheduledEvents.delete(key);
            }
        }
        
        // Schedule next check
        this.timeout = setTimeout(() => this.scheduleEvents(), 50);
    }
    
    watch() {
        console.log(`👁 Watching ${this.config.patternFile}`);
        
        fs.watchFile(this.config.patternFile, { interval: 100 }, (curr, prev) => {
            if (curr.mtime !== prev.mtime) {
                console.log('\n📝 Pattern changed');
                // Just reload pattern, don't restart playback
                this.loadPattern();
            }
        });
    }
}

// CLI
async function main() {
    const args = process.argv.slice(2);
    const command = args[0] || 'play';
    
    const config = {};
    
    // Parse arguments
    for (let i = 1; i < args.length; i++) {
        if (args[i] === '--bpm' && args[i+1]) {
            config.bpm = parseFloat(args[i+1]);
            i++;
        } else if (args[i] === '--cps' && args[i+1]) {
            config.cps = parseFloat(args[i+1]);
            i++;
        } else if (!args[i].startsWith('-')) {
            config.patternFile = args[i];
        }
    }
    
    const boson = new RealBoson(config);
    await boson.init();
    
    if (command === 'watch') {
        boson.watch();
        boson.play();
        
        process.stdin.resume();
        process.on('SIGINT', () => {
            console.log('\n👋 Bye');
            boson.stop();
            process.exit();
        });
    }
}

if (require.main === module) {
    main().catch(console.error);
}

module.exports = RealBoson;