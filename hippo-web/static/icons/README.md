# Hippo PWA Icons

This directory contains the icons used for the Progressive Web App.

## Icon Files

- `icon.svg` - Scalable vector icon (source)
- `icon-192.png` - 192x192 PNG for PWA manifest
- `icon-512.png` - 512x512 PNG for PWA manifest
- `apple-touch-icon.png` - 180x180 PNG for iOS
- `favicon.ico` - Multi-size ICO for browser tabs
- `favicon-16.png` - 16x16 favicon
- `favicon-32.png` - 32x32 favicon

## Generating PNG Icons

Run the provided script to generate all PNG icons from the SVG source:

```bash
cd hippo-web/static/icons
./generate-icons.sh
```

### Requirements

The script requires one of the following tools:

- **librsvg** (recommended): `brew install librsvg` (macOS) or `sudo apt install librsvg2-bin` (Linux)
- **ImageMagick**: `brew install imagemagick` (macOS) or `sudo apt install imagemagick` (Linux)
- **Inkscape**: Download from https://inkscape.org/

## Manual Generation

If you prefer to generate icons manually:

### Using rsvg-convert:
```bash
rsvg-convert -w 192 -h 192 icon.svg -o icon-192.png
rsvg-convert -w 512 -h 512 icon.svg -o icon-512.png
rsvg-convert -w 180 -h 180 icon.svg -o apple-touch-icon.png
```

### Using ImageMagick:
```bash
convert -background none -resize 192x192 icon.svg icon-192.png
convert -background none -resize 512x512 icon.svg icon-512.png
convert -background none -resize 180x180 icon.svg apple-touch-icon.png
```

### Favicon.ico:
```bash
convert favicon-16.png favicon-32.png favicon.ico
```

## Customization

To customize the icons:

1. Edit `icon.svg` with your preferred SVG editor (Inkscape, Figma, Adobe Illustrator, etc.)
2. Run `./generate-icons.sh` to regenerate all PNG files
3. Test the icons in the PWA by checking the manifest at `/manifest.json`

## Icon Guidelines

For best PWA compatibility:

- Icons should be square (1:1 aspect ratio)
- Use simple, recognizable designs
- Ensure good contrast for visibility
- Test on both light and dark backgrounds
- Consider the "maskable" icon safe zone (80% of the icon should be within the center circle)

## Current Design

The current icon features:
- A stylized hippo silhouette (representing memory/never forgetting)
- Indigo/purple theme (#6366f1) matching the app's brand color
- A document/memory symbol on the hippo's body
- Clean, minimal design suitable for app icons
