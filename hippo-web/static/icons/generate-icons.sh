#!/bin/bash
# Script to generate PNG icons from SVG
# Requires ImageMagick or librsvg (rsvg-convert)

# Check if rsvg-convert is available
if command -v rsvg-convert &> /dev/null; then
    echo "Using rsvg-convert to generate icons..."
    rsvg-convert -w 192 -h 192 icon.svg -o icon-192.png
    rsvg-convert -w 512 -h 512 icon.svg -o icon-512.png
    rsvg-convert -w 180 -h 180 icon.svg -o apple-touch-icon.png
    rsvg-convert -w 32 -h 32 icon.svg -o favicon-32.png
    rsvg-convert -w 16 -h 16 icon.svg -o favicon-16.png
    echo "Icons generated successfully!"

# Check if ImageMagick is available
elif command -v convert &> /dev/null; then
    echo "Using ImageMagick to generate icons..."
    convert -background none -resize 192x192 icon.svg icon-192.png
    convert -background none -resize 512x512 icon.svg icon-512.png
    convert -background none -resize 180x180 icon.svg apple-touch-icon.png
    convert -background none -resize 32x32 icon.svg favicon-32.png
    convert -background none -resize 16x16 icon.svg favicon-16.png
    echo "Icons generated successfully!"

# Check if inkscape is available
elif command -v inkscape &> /dev/null; then
    echo "Using Inkscape to generate icons..."
    inkscape icon.svg --export-filename=icon-192.png --export-width=192 --export-height=192
    inkscape icon.svg --export-filename=icon-512.png --export-width=512 --export-height=512
    inkscape icon.svg --export-filename=apple-touch-icon.png --export-width=180 --export-height=180
    inkscape icon.svg --export-filename=favicon-32.png --export-width=32 --export-height=32
    inkscape icon.svg --export-filename=favicon-16.png --export-width=16 --export-height=16
    echo "Icons generated successfully!"

else
    echo "Error: No SVG converter found!"
    echo "Please install one of the following:"
    echo "  - librsvg (rsvg-convert)"
    echo "  - ImageMagick (convert)"
    echo "  - Inkscape"
    echo ""
    echo "On macOS: brew install librsvg"
    echo "On Ubuntu/Debian: sudo apt install librsvg2-bin"
    exit 1
fi

# Generate favicon.ico (requires ImageMagick)
if command -v convert &> /dev/null; then
    convert favicon-16.png favicon-32.png favicon.ico
    echo "favicon.ico generated!"
else
    echo "Warning: Could not generate favicon.ico (ImageMagick not found)"
fi
