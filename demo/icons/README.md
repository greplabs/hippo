# Hippo Demo Icons

This directory should contain app icons for the demo.

## Required Icons

- `favicon.ico` - Browser favicon
- `favicon-16.png` - 16x16 favicon
- `favicon-32.png` - 32x32 favicon
- `icon-192.png` - 192x192 PWA icon
- `icon-512.png` - 512x512 PWA icon
- `apple-touch-icon.png` - 180x180 Apple touch icon

## Generating Icons

If you have the main Hippo icon SVG:

```bash
# Using ImageMagick
convert icon.svg -resize 16x16 favicon-16.png
convert icon.svg -resize 32x32 favicon-32.png
convert icon.svg -resize 192x192 icon-192.png
convert icon.svg -resize 512x512 icon-512.png
convert icon.svg -resize 180x180 apple-touch-icon.png

# Create ICO file
convert icon.svg -define icon:auto-resize=16,32,48,64,256 favicon.ico
```

Or use an online tool like [RealFaviconGenerator](https://realfavicongenerator.net/).

## Placeholder Icons

For testing, you can create simple placeholder icons:

```bash
# Purple square with "H" letter
convert -size 192x192 xc:'#6366f1' -fill white -gravity center \
  -pointsize 120 -font Arial-Bold -annotate +0+0 'H' icon-192.png

convert -size 512x512 xc:'#6366f1' -fill white -gravity center \
  -pointsize 320 -font Arial-Bold -annotate +0+0 'H' icon-512.png
```
