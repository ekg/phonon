/**
 * Phonon Modular Synthesis DSL - Live Coding Demo
 * 
 * This script demonstrates how to use the DSL from JavaScript/Strudel
 */

// Import OSC library for communication with Fermion
const osc = require('osc');

// Create OSC client
const oscClient = new osc.UDPPort({
    localAddress: "127.0.0.1",
    localPort: 57121,
    remoteAddress: "127.0.0.1",
    remotePort: 57120
});

oscClient.open();

// Helper function to send DSL patch
function loadDSL(patchText) {
    oscClient.send({
        address: "/dsl/load",
        args: [{ type: "s", value: patchText }]
    });
}

// Helper function to set bus value
function setBus(busName, value) {
    oscClient.send({
        address: "/dsl/bus/set",
        args: [
            { type: "s", value: busName },
            { type: "f", value: value }
        ]
    });
}

// Helper function to send pattern event
function patternEvent(patternName, value, time, duration, velocity) {
    oscClient.send({
        address: "/dsl/pattern/event",
        args: [
            { type: "s", value: patternName },
            { type: "s", value: value },
            { type: "f", value: time },
            { type: "f", value: duration },
            { type: "f", value: velocity }
        ]
    });
}

// Helper function to register pattern
function registerPattern(patternName) {
    oscClient.send({
        address: "/dsl/pattern/register",
        args: [{ type: "s", value: patternName }]
    });
}

// Helper function to add modulation route
function addRoute(routeString) {
    oscClient.send({
        address: "/dsl/route/add",
        args: [{ type: "s", value: routeString }]
    });
}

// === Demo 1: Basic Synthesis ===
console.log("Demo 1: Loading basic synthesis patch...");

const basicPatch = `
~lfo: sine(0.5) * 0.5 + 0.5
~osc: saw(220)
~filtered: ~osc >> lpf(~lfo * 2000 + 500, 0.7)
out: ~filtered * 0.3
`;

loadDSL(basicPatch);

// === Demo 2: Pattern Integration ===
setTimeout(() => {
    console.log("Demo 2: Adding patterns...");
    
    const patternPatch = `
~kick: "bd ~ ~ bd"
~snare: "~ sn ~ sn"
~hats: "hh*8" >> gain(0.2)
~drums: ~kick + ~snare + ~hats

~bass: saw(55) >> lpf(800, 0.8)
~bass_pattern: "c2 ~ e2 ~"

out: ~drums * 0.6 + ~bass * 0.4
`;
    
    loadDSL(patternPatch);
}, 3000);

// === Demo 3: Cross-Modulation ===
setTimeout(() => {
    console.log("Demo 3: Cross-modulation example...");
    
    const crossModPatch = `
// Bass synthesis
~bass_env: perc(0.01, 0.3)
~bass: saw(55) * ~bass_env >> lpf(1000, 0.8)

// Extract bass features
~bass_rms: ~bass >> rms(0.05)
~bass_transient: ~bass >> transient

// Drums modulated by bass
~kick: "bd ~ ~ bd"
~kick_transient: ~kick >> transient
~snare: "~ sn ~ sn" >> lpf(~bass_rms * 3000 + 1000, 0.7)
~hats: "hh*16" >> hpf(~bass_rms * 6000 + 2000, 0.8) >> gain(0.2)

// Sidechain compression
~bass_ducked: ~bass * (1 - ~kick_transient * 0.5)

// Mix
~mix: ~kick * 0.5 + ~snare * 0.3 + ~hats * 0.2 + ~bass_ducked * 0.4
out: ~mix >> compress(0.3, 4)
`;
    
    loadDSL(crossModPatch);
}, 6000);

// === Demo 4: Live Control ===
setTimeout(() => {
    console.log("Demo 4: Live parameter control...");
    
    // Register patterns for live control
    registerPattern("melody");
    registerPattern("bass");
    
    // Send some pattern events
    let noteIndex = 0;
    const notes = ["c4", "e4", "g4", "c5"];
    
    setInterval(() => {
        // Send melody note
        patternEvent("melody", notes[noteIndex], 0, 0.25, 0.7);
        noteIndex = (noteIndex + 1) % notes.length;
        
        // Modulate filter cutoff with LFO
        const lfoValue = Math.sin(Date.now() * 0.001) * 0.5 + 0.5;
        setBus("~filter_mod", lfoValue);
    }, 250);
    
    // Add modulation routes
    addRoute("~melody -> ~reverb.mix: 0.3");
    addRoute("~bass -> ~filter.cutoff: 0.5");
    
}, 9000);

// === Demo 5: Complex Routing ===
setTimeout(() => {
    console.log("Demo 5: Complex modular routing...");
    
    const complexPatch = `
// Multiple LFOs at different rates
~lfo1: sine(0.25) * 0.5 + 0.5
~lfo2: sine(3) * 0.3
~lfo3: sine(0.1) * 0.5 + 0.5

// Oscillator bank
~osc1: saw(110) * ~lfo1
~osc2: square(220 + ~lfo2 * 20)
~osc3: triangle(330)

// Parallel filtering
~low: ~osc1 >> lpf(800, 0.8)
~band: ~osc2 >> bpf(1500, 0.5)
~high: ~osc3 >> hpf(2000, 0.7)

// Mix with cross-modulation
~mix: ~low * (1 - ~lfo3 * 0.3) + 
      ~band * ~lfo1 + 
      ~high * 0.2

// Effects chain
~delayed: ~mix >> delay(0.333) >> lpf(3000, 0.5)
~reverbed: ~delayed >> reverb(0.7, 0.8)

// Master
~master: (~mix * 0.7 + ~reverbed * 0.3) >> compress(0.3, 4) >> limit(0.95)
out: ~master
`;
    
    loadDSL(complexPatch);
}, 12000);

// === Interactive Controls ===
console.log("\nInteractive controls:");
console.log("- Use setBus('~busname', value) to control parameters");
console.log("- Use patternEvent() to trigger pattern events");
console.log("- Use addRoute() to add modulation routes");
console.log("- Use loadDSL() to hot-swap patches\n");

// Export functions for REPL use
module.exports = {
    loadDSL,
    setBus,
    patternEvent,
    registerPattern,
    addRoute,
    oscClient
};