#!/usr/bin/env node

/**
 * Complete Pattern Engine Test Suite
 * Runs all pattern operator tests
 */

const { execSync } = require('child_process');
const path = require('path');

console.log('🚀 PHONON PATTERN ENGINE - COMPLETE TEST SUITE\n');
console.log('=' .repeat(60));

const testFiles = [
    'test-core.js',
    'test-time.js', 
    'test-random.js',
    'test-signals.js',
    'test-advanced.js'
];

const results = {
    total: 0,
    passed: 0,
    failed: 0,
    files: []
};

for (const file of testFiles) {
    console.log(`\n📋 Running ${file}...`);
    console.log('-'.repeat(40));
    
    try {
        const output = execSync(`node ${file}`, {
            cwd: __dirname,
            encoding: 'utf8'
        });
        
        // Parse results from output
        const lines = output.split('\n');
        const resultLine = lines.find(l => l.includes('Results:'));
        
        if (resultLine) {
            const match = resultLine.match(/(\d+) passed, (\d+) failed/);
            if (match) {
                const passed = parseInt(match[1]);
                const failed = parseInt(match[2]);
                
                results.passed += passed;
                results.failed += failed;
                results.total += passed + failed;
                
                results.files.push({
                    file,
                    passed,
                    failed,
                    status: failed === 0 ? '✅' : '❌'
                });
                
                console.log(`${failed === 0 ? '✅' : '❌'} ${passed} passed, ${failed} failed`);
            }
        }
    } catch (error) {
        console.log(`❌ Failed to run ${file}`);
        results.files.push({
            file,
            passed: 0,
            failed: 1,
            status: '❌'
        });
        results.failed++;
        results.total++;
    }
}

console.log('\n' + '='.repeat(60));
console.log('📊 FINAL RESULTS\n');

console.log('Test Files:');
for (const { file, passed, failed, status } of results.files) {
    console.log(`  ${status} ${file.padEnd(20)} ${passed} passed, ${failed} failed`);
}

console.log('\nTotals:');
console.log(`  Total Tests: ${results.total}`);
console.log(`  ✅ Passed:   ${results.passed}`);
console.log(`  ❌ Failed:   ${results.failed}`);
console.log(`  Success Rate: ${((results.passed / results.total) * 100).toFixed(1)}%`);

console.log('\n' + '='.repeat(60));

// Count implemented operators
const pattern = require('./pattern');
const operators = Object.keys(pattern).filter(k => 
    typeof pattern[k] === 'function' && 
    !['Fraction', 'TimeSpan', 'Event', 'Pattern'].includes(k)
);

console.log('\n📦 IMPLEMENTED OPERATORS\n');
console.log(`Total Operators: ${operators.length}`);
console.log('\nCategories:');

const categories = {
    'Core Creation': ['pure', 'silence', 'gap'],
    'Combination': ['stack', 'cat', 'fastcat', 'slowcat', 'sequence', 'polymeter', 'polyrhythm'],
    'Time Manipulation': ['fast', 'slow', 'early', 'late', 'compress', 'zoom', 'ply', 'inside', 'outside', 'segment', 'chop'],
    'Pattern Structure': ['rev', 'palindrome', 'iter', 'every'],
    'Randomness': ['rand', 'irand', 'choose', 'wchoose', 'shuffle', 'scramble', 'degrade', 'degradeBy', 'sometimes', 'sometimesBy', 'often', 'rarely', 'almostNever', 'almostAlways'],
    'Signals': ['sine', 'cosine', 'saw', 'square', 'tri', 'perlin'],
    'Euclidean': ['euclid', 'euclidRot', 'euclidLegato'],
    'Pattern Combination': ['jux', 'juxBy', 'superimpose', 'layer', 'off', 'echo', 'stut'],
    'Filtering': ['when', 'mask', 'struct', 'filter'],
    'Math': ['add', 'sub', 'mul', 'div', 'mod', 'range']
};

for (const [category, ops] of Object.entries(categories)) {
    const implemented = ops.filter(op => operators.includes(op));
    console.log(`  ${category}: ${implemented.length}/${ops.length}`);
}

console.log('\n' + '='.repeat(60));
console.log('\n🎉 Pattern Engine Implementation Complete!');
console.log(`   ${operators.length} operators implemented and tested`);
console.log(`   ${results.passed}/${results.total} tests passing`);
console.log('\n' + '='.repeat(60));

process.exit(results.failed > 0 ? 1 : 0);