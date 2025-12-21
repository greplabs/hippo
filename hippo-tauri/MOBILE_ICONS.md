# Mobile Icon Generation Guide

This guide explains how to generate mobile app icons for Hippo on iOS and Android.

## iOS Icons

iOS requires multiple icon sizes for different devices and use cases. After running `make mobile-init`, Tauri will generate a default icon set, but you should replace them with proper Hippo branding.

### Required Sizes (iOS)

Place these in `gen/ios/Assets.xcassets/AppIcon.appiconset/`:

- **20x20** (2x, 3x) - Notification icons
- **29x29** (2x, 3x) - Settings icons
- **40x40** (2x, 3x) - Spotlight search
- **60x60** (2x, 3x) - Home screen (iPhone)
- **76x76** (1x, 2x) - Home screen (iPad)
- **83.5x83.5** (2x) - Home screen (iPad Pro)
- **1024x1024** (1x) - App Store

### Generate iOS Icons

```bash
# Using ImageMagick (install with: brew install imagemagick)
cd hippo-tauri/icons

# From your source icon (must be 1024x1024 or larger)
SOURCE="icon.png"

# Generate all sizes
convert $SOURCE -resize 20x20 ios-20.png
convert $SOURCE -resize 40x40 ios-20@2x.png
convert $SOURCE -resize 60x60 ios-20@3x.png
convert $SOURCE -resize 29x29 ios-29.png
convert $SOURCE -resize 58x58 ios-29@2x.png
convert $SOURCE -resize 87x87 ios-29@3x.png
convert $SOURCE -resize 40x40 ios-40.png
convert $SOURCE -resize 80x80 ios-40@2x.png
convert $SOURCE -resize 120x120 ios-40@3x.png
convert $SOURCE -resize 120x120 ios-60@2x.png
convert $SOURCE -resize 180x180 ios-60@3x.png
convert $SOURCE -resize 76x76 ios-76.png
convert $SOURCE -resize 152x152 ios-76@2x.png
convert $SOURCE -resize 167x167 ios-83.5@2x.png
convert $SOURCE -resize 1024x1024 ios-1024.png
```

Or use an online tool like [AppIconGenerator](https://www.appicon.co/).

## Android Icons

Android uses a more flexible icon system with adaptive icons.

### Required Sizes (Android)

Place these in `gen/android/app/src/main/res/`:

- **mipmap-mdpi/** - 48x48px
- **mipmap-hdpi/** - 72x72px
- **mipmap-xhdpi/** - 96x96px
- **mipmap-xxhdpi/** - 144x144px
- **mipmap-xxxhdpi/** - 192x192px

### Generate Android Icons

```bash
cd hippo-tauri/icons
SOURCE="icon.png"

# Generate all densities
convert $SOURCE -resize 48x48 android-mdpi.png
convert $SOURCE -resize 72x72 android-hdpi.png
convert $SOURCE -resize 96x96 android-xhdpi.png
convert $SOURCE -resize 144x144 android-xxhdpi.png
convert $SOURCE -resize 192x192 android-xxxhdpi.png
```

### Adaptive Icons (Android 8.0+)

For modern Android devices, create adaptive icons with separate foreground and background layers:

1. **Foreground layer** - Your app logo (108x108dp safe area)
2. **Background layer** - Solid color or pattern

Place in `gen/android/app/src/main/res/mipmap-anydpi-v26/`:
- `ic_launcher.xml`
- `ic_launcher_round.xml`

Example `ic_launcher.xml`:
```xml
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
</adaptive-icon>
```

## Automated Generation

The easiest way is to use Tauri's built-in icon generation:

```bash
# Install tauri-cli if not already installed
cargo install tauri-cli

# Generate all mobile icons from a source icon
cd hippo-tauri
cargo tauri icon icons/icon.png
```

This will automatically generate all required sizes for both iOS and Android.

## Current Icons

The project currently has these icons:
- `icons/icon.png` (2202 bytes) - Main app icon
- `icons/32x32.png`, `icons/128x128.png`, `icons/128x128@2x.png` - Desktop sizes
- `icons/icon.ico` - Windows icon

For production mobile apps, you should:
1. Create a high-resolution source icon (1024x1024 minimum)
2. Run `cargo tauri icon icons/source-icon.png`
3. Manually adjust any platform-specific variations

## Testing Icons

### iOS
```bash
make mobile-ios
# Icons appear on the simulator home screen
```

### Android
```bash
make mobile-android
# Icons appear on the emulator app drawer
```

## Resources

- [iOS Human Interface Guidelines - App Icons](https://developer.apple.com/design/human-interface-guidelines/app-icons)
- [Android App Icons](https://developer.android.com/guide/practices/ui_guidelines/icon_design_launcher)
- [Tauri Icon Documentation](https://v2.tauri.app/reference/cli/#icon)
