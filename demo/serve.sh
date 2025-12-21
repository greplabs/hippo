#!/bin/bash
# Simple script to serve the Hippo demo locally

PORT="${1:-8080}"

echo "🦛 Starting Hippo Demo Server..."
echo "📂 Serving from: $(pwd)"
echo "🌐 URL: http://localhost:$PORT"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Try different methods to serve
if command -v python3 &> /dev/null; then
    echo "Using Python 3..."
    python3 -m http.server $PORT
elif command -v python &> /dev/null; then
    echo "Using Python 2..."
    python -m SimpleHTTPServer $PORT
elif command -v php &> /dev/null; then
    echo "Using PHP..."
    php -S localhost:$PORT
elif command -v npx &> /dev/null; then
    echo "Using npx http-server..."
    npx http-server -p $PORT
else
    echo "❌ Error: No suitable HTTP server found."
    echo ""
    echo "Please install one of:"
    echo "  - Python 3: brew install python3"
    echo "  - Node.js: brew install node"
    echo "  - PHP: brew install php"
    exit 1
fi
