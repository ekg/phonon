#!/usr/bin/env node

/**
 * Download Strudel/Dirt samples for Phonon Forge
 * 
 * Sample naming convention in Strudel:
 * - "bd" plays bd/BT0A0A7.wav (first file in bd folder)
 * - "bd:1" plays the second file in bd folder
 * - "bd:2" plays the third file, etc.
 */

const https = require('https');
const fs = require('fs');
const path = require('path');

// Base URL for Dirt-Samples
const BASE_URL = 'https://raw.githubusercontent.com/tidalcycles/Dirt-Samples/master';

// Essential drum samples to download
// Based on the Dirt-Samples repository structure
const SAMPLE_MAP = {
    // Kicks
    'bd': [
        'BT0A0A7.wav',  // bd:0
        'BT0AAD0.wav',  // bd:1
        'BT0AADA.wav',  // bd:2
        'BT0ADA0.wav',  // bd:3
        'BT0ADAA.wav',  // bd:4
    ],
    
    // Snares
    'sn': [
        'ST0T0S0.wav',  // sn:0
        'ST0T0S3.wav',  // sn:1
        'ST0T0S7.wav',  // sn:2
        'ST0TAS0.wav',  // sn:3
        'ST0TAS3.wav',  // sn:4
    ],
    
    // Hi-hats
    'hh': [
        'HH0-000.wav',  // hh:0
        'HH0-001.wav',  // hh:1
        'HH0-002.wav',  // hh:2
        'HH0-003.wav',  // hh:3
        'HH0-004.wav',  // hh:4
    ],
    
    // Claps
    'cp': [
        'HANDCLAP0.wav', // cp:0
        'HANDCLP1.wav',  // cp:1
        'HANDCLP2.wav',  // cp:2
    ],
    
    // Cymbals/Crashes
    'cr': [
        '001_CRASHCYMBAL.wav', // cr:0
        '002_CRASHCYMBAL.wav', // cr:1
    ],
    
    // Open hi-hat
    'oh': [
        'HH1-001.wav',  // oh:0
        'HH1-002.wav',  // oh:1
        'HH1-003.wav',  // oh:2
    ],
    
    // Rimshot
    'rs': [
        'SIDESTICK.wav', // rs:0
    ],
    
    // Cowbell
    'cb': [
        'COWBELL1.wav',  // cb:0
    ],
    
    // Bass sounds (808-like)
    'bass': [
        'BASS0.wav',     // bass:0
        'BASS1.wav',     // bass:1
        'BASS2.wav',     // bass:2
    ]
};

function downloadFile(url, dest) {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(dest);
        
        https.get(url, (response) => {
            if (response.statusCode !== 200) {
                reject(new Error(`Failed to download ${url}: ${response.statusCode}`));
                return;
            }
            
            response.pipe(file);
            
            file.on('finish', () => {
                file.close();
                resolve();
            });
        }).on('error', (err) => {
            fs.unlink(dest, () => {}); // Delete partial file
            reject(err);
        });
    });
}

async function downloadSamples() {
    const samplesDir = path.join(__dirname, 'samples');
    
    console.log('📦 Downloading Strudel/Dirt samples...\n');
    
    for (const [folder, files] of Object.entries(SAMPLE_MAP)) {
        const folderPath = path.join(samplesDir, folder);
        
        // Create folder
        if (!fs.existsSync(folderPath)) {
            fs.mkdirSync(folderPath, { recursive: true });
        }
        
        console.log(`📁 ${folder}/`);
        
        for (let i = 0; i < files.length; i++) {
            const filename = files[i];
            const url = `${BASE_URL}/${folder}/${filename}`;
            const dest = path.join(folderPath, filename);
            
            // Skip if already exists
            if (fs.existsSync(dest)) {
                console.log(`  ✓ ${filename} (exists)`);
                continue;
            }
            
            try {
                process.stdout.write(`  ⬇ ${filename}...`);
                await downloadFile(url, dest);
                console.log(' ✓');
            } catch (err) {
                console.log(` ✗ ${err.message}`);
            }
        }
    }
    
    console.log('\n✅ Sample download complete!');
    console.log('\nUsage in patterns:');
    console.log('  "bd"     - plays bd/BT0A0A7.wav');
    console.log('  "bd:1"   - plays bd/BT0AAD0.wav');
    console.log('  "sn:2"   - plays sn/ST0T0S7.wav');
    console.log('  etc.\n');
}

// Run if called directly
if (require.main === module) {
    downloadSamples().catch(console.error);
}

module.exports = { SAMPLE_MAP, downloadFile };