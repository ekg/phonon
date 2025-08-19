#!/bin/bash

# Install actual Strudel packages for proper mini notation support

echo "📦 Installing Strudel packages for Phonon..."
echo ""

cd "$(dirname "$0")/boson"

# Check if npm is available
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed!"
    echo "Please install Node.js/npm first:"
    echo "  pkg install nodejs"
    exit 1
fi

echo "Installing Strudel core packages..."
npm install @strudel/core @strudel/mini @strudel/transpiler

echo ""
echo "✅ Strudel packages installed!"
echo ""
echo "Now you can use proper mini notation:"
echo '  stack("bd*4", "~ cp ~ cp", "hh*8")'
echo '  "<c3 e3 g3> <f3 a3 c4>"'
echo '  "bd(5,8)"'
echo '  "[bd,c2]*2 [cp,e2]"'
echo ""
echo "Run with: node boson/boson-strudel.js watch"