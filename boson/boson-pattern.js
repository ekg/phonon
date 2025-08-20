#!/usr/bin/env node

/**
 * Boson with REAL pattern engine integration
 */

const OSC = require('osc-js');
const fs = require('fs');
const path = require('path');

// Import the ACTUAL pattern engine we built!
const {
    Pattern,
    pure,
    stack,
    cat,
    euclid,
    fast,
    slow,
    every,
    sometimes,
    degrade,
    rev,
    // ... we have 150 operators available!
} = require('./pattern');

class BetterBoson {
    constructor(config = {}) {
        this.config = {
            oscPort: config.oscPort || 57120,
            oscHost: config.oscHost || 'localhost',
            tempo: config.tempo || 120,
            cps: config.cps || 0.5, // Cycles per second (120bpm = 0.5 cps)
            patternFile: config.patternFile || '../patterns.phonon',
            ...config
        };
        
        this.osc = new OSC({ plugin: new OSC.DatagramPlugin() });
        this.pattern = null;
        this.playing = false;
        this.startTime = null;
        
        console.log('🎼 Boson Pattern Engine (with REAL patterns!)');
        console.log(`   OSC: ${this.config.oscHost}:${this.config.oscPort}`);
        console.log(`   Tempo: ${this.config.tempo} BPM (${this.config.cps} cps)`);
        console.log(`   Pattern file: ${this.config.patternFile}`);
    }
    
    async init() {
        await this.osc.open({ port: this.config.oscPort + 1 });
        console.log('✓ OSC ready');
        this.loadPattern();
    }
    
    /**
     * Parse mini-notation into actual Pattern objects
     */
    parseMiniNotation(str) {
        // Remove quotes if present
        str = str.replace(/^["']|["']$/g, '').trim();
        
        // Handle stacks (comma-separated)
        if (str.includes(',')) {
            const parts = str.split(',').map(p => this.parseMiniNotation(p.trim()));
            return stack(...parts);
        }
        
        // Handle euclidean rhythms: bd(3,8)
        const euclidMatch = str.match(/^(\w+)\((\d+),(\d+)(?:,(\d+))?\)$/);
        if (euclidMatch) {
            const [, sample, k, n, rot] = euclidMatch;
            const pattern = euclid(parseInt(k), parseInt(n), parseInt(rot) || 0);
            return pattern.fmap(_ => ({ type: 'sample', name: sample }));
        }
        
        // Handle repetition: bd*4
        const repeatMatch = str.match(/^(\w+)\*(\d+)$/);
        if (repeatMatch) {
            const [, sample, count] = repeatMatch;
            return fast(parseInt(count), pure({ type: 'sample', name: sample }));
        }
        
        // Parse space-separated tokens
        const tokens = str.split(/\s+/);
        if (tokens.length === 0) return pure(null);
        
        const patterns = tokens.map(token => {
            if (token === '~' || token === '.') {
                return pure(null); // Rest
            } else if (token.includes('*')) {
                // Handle inline repetition
                const [sample, count] = token.split('*');
                return fast(parseInt(count) || 1, pure({ type: 'sample', name: sample }));
            } else {
                return pure({ type: 'sample', name: token });
            }
        });
        
        // Concatenate patterns in sequence
        return patterns.length === 1 ? patterns[0] : cat(...patterns);
    }
    
    loadPattern() {
        try {
            const content = fs.readFileSync(this.config.patternFile, 'utf8');
            
            // Extract pattern string from file
            const match = content.match(/"([^"]+)"/);
            if (!match) {
                console.log('No pattern found in file');
                return false;
            }
            
            // Parse using mini-notation parser
            this.pattern = this.parseMiniNotation(match[1]);
            console.log(`✓ Pattern loaded: ${match[1]}`);
            return true;
            
        } catch (err) {
            console.error('Failed to load pattern:', err);
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
        
        console.log('▶ Playing pattern');
        this.scheduleNextWindow();
    }
    
    stop() {
        this.playing = false;
        if (this.scheduledTimeout) {
            clearTimeout(this.scheduledTimeout);
        }
        console.log('■ Stopped');
    }
    
    scheduleNextWindow() {
        if (!this.playing) return;
        
        const now = Date.now() / 1000;
        const elapsed = now - this.startTime;
        const currentCycle = elapsed * this.config.cps;
        
        // Query pattern for next small window
        const windowSize = 0.1; // Query 100ms ahead
        const events = this.pattern.queryArc(currentCycle, currentCycle + windowSize);
        
        // Send OSC messages for each event
        for (const event of events) {
            if (event.value && event.value.type === 'sample') {
                const message = new OSC.Message('/sample', event.value.name, 0, 1.0);
                this.osc.send(message, {
                    port: this.config.oscPort,
                    host: this.config.oscHost
                });
                
                // Calculate when this event should play
                const eventTime = event.part.begin.toFloat() / this.config.cps;
                const delay = Math.max(0, (this.startTime + eventTime - now) * 1000);
                
                if (delay < 100) { // Only log imminent events
                    console.log(`  ♫ ${event.value.name} @ ${event.part.begin.toFloat().toFixed(3)}`);
                }
            }
        }
        
        // Schedule next window
        this.scheduledTimeout = setTimeout(() => {
            this.scheduleNextWindow();
        }, 50); // Check every 50ms
    }
    
    watch() {
        console.log(`👁 Watching ${this.config.patternFile}`);
        
        fs.watchFile(this.config.patternFile, { interval: 100 }, (curr, prev) => {
            if (curr.mtime !== prev.mtime) {
                console.log('\n📝 Pattern file changed');
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
    if (args[1] && !args[1].startsWith('-')) {
        config.patternFile = args[1];
    }
    
    const boson = new BetterBoson(config);
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

module.exports = BetterBoson;