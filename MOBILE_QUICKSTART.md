# Hippo Mobile Apps - Quick Start Checklist

Follow these steps to get your first mobile build running.

## Prerequisites Checklist

### For Android (macOS, Linux, or Windows)

- [ ] Java JDK 17+ installed
  ```bash
  java -version
  ```
- [ ] Android Studio installed
- [ ] Android SDK installed (API 24+)
- [ ] Android NDK installed (recommended: 26.1.10909125)
- [ ] Environment variables set:
  ```bash
  echo $ANDROID_HOME
  echo $NDK_HOME
  ```
- [ ] Rust Android targets installed:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi
  ```

### For iOS (macOS only)

- [ ] Xcode 13+ installed from App Store
- [ ] Xcode Command Line Tools installed:
  ```bash
  xcode-select --install
  ```
- [ ] CocoaPods installed:
  ```bash
  pod --version
  ```
- [ ] Rust iOS targets installed:
  ```bash
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim
  ```
- [ ] Apple Developer account (free tier OK for testing)
- [ ] Team ID from Apple Developer portal

## Setup Steps

### Step 1: Initialize Mobile Projects

```bash
cd /Users/punitmishra/Downloads/hippov20
make mobile-init
```

This creates:
- `hippo-tauri/gen/android/` - Android project
- `hippo-tauri/gen/ios/` - iOS Xcode project

### Step 2: Configure iOS Signing (iOS only)

Edit `hippo-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "iOS": {
      "developmentTeam": "YOUR_TEAM_ID"
    }
  }
}
```

Find your Team ID:
1. Open Xcode
2. Preferences > Accounts
3. Select your Apple ID
4. Copy the Team ID shown

### Step 3: Merge Platform Permissions

#### iOS Permissions
```bash
# Open gen/ios/Hippo/Info.plist in Xcode
# Manually add entries from hippo-tauri/Info.plist.additions
```

Key permissions to add:
- NSPhotoLibraryUsageDescription
- NSCameraUsageDescription
- NSMicrophoneUsageDescription
- NSDocumentsFolderUsageDescription

#### Android Permissions
```bash
# Edit gen/android/app/src/main/AndroidManifest.xml
# Add permissions from hippo-tauri/AndroidManifest.xml.additions
```

Then copy FileProvider config:
```bash
mkdir -p hippo-tauri/gen/android/app/src/main/res/xml/
cp hippo-tauri/android-file-paths.xml \
   hippo-tauri/gen/android/app/src/main/res/xml/file_paths.xml
```

### Step 4: Generate Icons (Optional but Recommended)

```bash
cd hippo-tauri
cargo tauri icon icons/icon.png
```

This generates all required icon sizes for both platforms.

### Step 5: Run Your First Build

#### Android
```bash
# Make sure emulator is running or device is connected
adb devices

# Run the app
make mobile-android
```

#### iOS
```bash
# Run on simulator
make mobile-ios
```

## Troubleshooting

### Android: "ANDROID_HOME not set"

Add to `~/.zshrc` or `~/.bashrc`:
```bash
export ANDROID_HOME=$HOME/Library/Android/sdk  # macOS
export ANDROID_HOME=$HOME/Android/Sdk          # Linux
export NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125
```

Then:
```bash
source ~/.zshrc  # or ~/.bashrc
```

### Android: "No devices found"

Start an emulator:
```bash
# List available
emulator -list-avds

# Start one
emulator -avd Pixel_5_API_33 &
```

Or connect a physical device:
1. Enable Developer Options
2. Enable USB Debugging
3. Connect via USB
4. Accept on device

### iOS: "No Development Team"

You forgot Step 2. Add your Team ID to `tauri.conf.json`.

### iOS: "Provisioning profile errors"

1. Open `hippo-tauri/gen/ios/Hippo.xcodeproj` in Xcode
2. Select Hippo target
3. Go to Signing & Capabilities
4. Enable "Automatically manage signing"
5. Select your team

### Build fails with Rust errors

```bash
# Update Rust
rustup update

# Clean build
cd hippo-tauri
cargo clean
make mobile-init
```

## Next Steps After First Build

- [ ] Test on real device (not just emulator/simulator)
- [ ] Test file indexing
- [ ] Test search functionality
- [ ] Test thumbnail generation
- [ ] Review UI on mobile screen sizes
- [ ] Test permissions (photos, camera, etc.)
- [ ] Build release version
- [ ] Test release build

## Building for Release

### Android APK
```bash
make mobile-android-build
```

Output: `hippo-tauri/gen/android/app/build/outputs/apk/release/`

### iOS IPA
```bash
make mobile-ios-build
```

Output: `hippo-tauri/gen/ios/build/Release-iphoneos/`

## Complete Documentation

- **MOBILE_SETUP.md** - Comprehensive setup guide
- **MOBILE_ICONS.md** - Icon generation guide
- **MOBILE_README.md** - Quick reference
- **MOBILE_CONFIGURATION_SUMMARY.md** - All changes made

## Common Commands

```bash
# Initialize (one time)
make mobile-init

# Development builds
make mobile-android    # Android dev
make mobile-ios        # iOS dev

# Production builds
make mobile-android-build  # Android release
make mobile-ios-build      # iOS release

# Clean
make mobile-clean

# Help
make mobile-help
```

## Getting Help

1. Check troubleshooting in MOBILE_SETUP.md
2. Review Tauri docs: https://v2.tauri.app/develop/mobile/
3. Check platform-specific docs:
   - Android: https://developer.android.com/
   - iOS: https://developer.apple.com/

## Success Indicators

You'll know it's working when:

✅ `make mobile-init` completes without errors
✅ `gen/android/` and `gen/ios/` directories are created
✅ `make mobile-android` opens app in emulator
✅ `make mobile-ios` opens app in simulator
✅ App launches and shows Hippo UI
✅ File picker works
✅ Search functionality works

## Time Estimate

- Prerequisites installation: 30-60 minutes
- First initialization: 5-10 minutes
- First successful build: 10-15 minutes
- Total for experienced developer: ~45-60 minutes
- Total for first-time mobile dev: 2-3 hours

Good luck! 🦛
