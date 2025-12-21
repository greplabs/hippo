# Hippo Mobile Apps Setup Guide

This guide walks you through setting up and building Hippo for iOS and Android using Tauri 2's native mobile support.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Initial Setup](#initial-setup)
3. [iOS Development](#ios-development)
4. [Android Development](#android-development)
5. [Building for Production](#building-for-production)
6. [Troubleshooting](#troubleshooting)

## Prerequisites

### All Platforms

1. **Rust** (1.70 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Node.js** (18 or later) and npm
   ```bash
   # macOS
   brew install node

   # Or download from https://nodejs.org/
   ```

3. **Tauri CLI**
   ```bash
   cargo install tauri-cli
   # Or via npm
   npm install -g @tauri-apps/cli
   ```

4. **Rust Mobile Targets**
   ```bash
   # Android targets
   rustup target add aarch64-linux-android
   rustup target add armv7-linux-androideabi
   rustup target add i686-linux-android
   rustup target add x86_64-linux-android

   # iOS targets (macOS only)
   rustup target add aarch64-apple-ios
   rustup target add aarch64-apple-ios-sim
   rustup target add x86_64-apple-ios
   ```

### Android-Specific

1. **Java Development Kit (JDK) 17+**
   ```bash
   # macOS
   brew install openjdk@17

   # Linux (Ubuntu/Debian)
   sudo apt install openjdk-17-jdk

   # Windows
   # Download from https://adoptium.net/
   ```

2. **Android Studio**
   - Download from https://developer.android.com/studio
   - Install Android SDK (API 24 or higher)
   - Install Android NDK (via SDK Manager)
   - Install Android SDK Build-Tools
   - Install Android SDK Platform-Tools

3. **Environment Variables**

   Add to your `~/.bashrc`, `~/.zshrc`, or `~/.profile`:

   ```bash
   # macOS/Linux
   export ANDROID_HOME=$HOME/Android/Sdk  # or ~/Library/Android/sdk on macOS
   export NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125  # Adjust version
   export PATH=$PATH:$ANDROID_HOME/platform-tools
   export PATH=$PATH:$ANDROID_HOME/tools
   export PATH=$PATH:$ANDROID_HOME/tools/bin
   export PATH=$PATH:$NDK_HOME
   ```

   ```powershell
   # Windows (PowerShell)
   $env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
   $env:NDK_HOME = "$env:ANDROID_HOME\ndk\26.1.10909125"
   ```

4. **Verify Android Setup**
   ```bash
   # Check Java
   java -version

   # Check Android tools
   adb --version
   sdkmanager --list
   ```

### iOS-Specific (macOS Only)

1. **Xcode 13.0+**
   - Install from Mac App Store
   - Install Command Line Tools:
     ```bash
     xcode-select --install
     ```

2. **CocoaPods**
   ```bash
   sudo gem install cocoapods
   ```

3. **iOS Simulator**
   - Open Xcode
   - Go to Preferences > Components
   - Download desired iOS Simulator versions

4. **Apple Developer Account**
   - Free account for development/testing
   - Paid account ($99/year) required for App Store distribution

## Initial Setup

### 1. Initialize Mobile Targets

From the project root:

```bash
make mobile-init
```

Or manually:

```bash
cd hippo-tauri
cargo tauri android init
cargo tauri ios init
```

This creates:
- `gen/android/` - Android project files
- `gen/ios/` - iOS/Xcode project files

### 2. Configure App Identifiers

The app is already configured with:
- **Bundle ID**: `com.greplabs.hippo`
- **Product Name**: Hippo
- **Version**: 0.1.0

To change these, edit `tauri.conf.json`:
```json
{
  "identifier": "com.greplabs.hippo",
  "productName": "Hippo",
  "version": "0.1.0"
}
```

### 3. Generate Icons

See [MOBILE_ICONS.md](./MOBILE_ICONS.md) for detailed instructions.

Quick generation:
```bash
cd hippo-tauri
cargo tauri icon icons/icon.png
```

### 4. Install Dependencies

```bash
cd hippo-tauri/ui
npm install
```

## iOS Development

### 1. Configure Development Team

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
- Open Xcode
- Go to Preferences > Accounts
- Select your Apple ID
- Your Team ID is shown next to your team name

### 2. Run on Simulator

```bash
make mobile-ios
```

Or:
```bash
cd hippo-tauri
cargo tauri ios dev
```

This will:
1. Build the Rust core
2. Build the UI
3. Generate iOS project
4. Open Xcode
5. Launch in iOS Simulator

### 3. Run on Device

1. Connect your iPhone/iPad via USB
2. Open `hippo-tauri/gen/ios/Hippo.xcodeproj` in Xcode
3. Select your device in the destination menu
4. Enable "Automatically manage signing" in Signing & Capabilities
5. Click Run (⌘R)

### 4. Debugging

View logs in Xcode's console or Terminal:
```bash
xcrun simctl spawn booted log stream --predicate 'process == "Hippo"'
```

## Android Development

### 1. Create/Connect Device

**Emulator:**
```bash
# List available AVDs
avdmanager list avd

# Create new AVD
avdmanager create avd -n Pixel_5 -k "system-images;android-33;google_apis;x86_64"

# Start emulator
emulator -avd Pixel_5
```

**Physical Device:**
1. Enable Developer Options on your Android device
2. Enable USB Debugging
3. Connect via USB
4. Verify: `adb devices`

### 2. Run on Emulator/Device

```bash
make mobile-android
```

Or:
```bash
cd hippo-tauri
cargo tauri android dev
```

This will:
1. Build the Rust core for Android
2. Build the UI
3. Generate Android project
4. Install and run on connected device/emulator

### 3. Debugging

View logs:
```bash
# All logs
adb logcat

# Filter for Hippo
adb logcat | grep -i hippo

# Rust panic logs
adb logcat | grep RustStdoutStderr
```

### 4. Android Studio

For advanced debugging, open the project in Android Studio:
```bash
cd hippo-tauri/gen/android
open -a "Android Studio" .
# or
studio .
```

## Building for Production

### iOS Production Build

```bash
make mobile-ios-build
```

Or:
```bash
cd hippo-tauri
cargo tauri ios build --release
```

Output: `hippo-tauri/gen/ios/build/Release-iphoneos/Hippo.ipa`

**For App Store:**
1. Open project in Xcode
2. Configure signing with Distribution certificate
3. Archive (Product > Archive)
4. Upload to App Store Connect

### Android Production Build

```bash
make mobile-android-build
```

Or:
```bash
cd hippo-tauri
cargo tauri android build --release
```

Output:
- APK: `hippo-tauri/gen/android/app/build/outputs/apk/release/`
- AAB: `hippo-tauri/gen/android/app/build/outputs/bundle/release/`

**For Play Store:**
1. Create a keystore (if you haven't):
   ```bash
   keytool -genkey -v -keystore hippo-release.keystore \
     -alias hippo -keyalg RSA -keysize 2048 -validity 10000
   ```

2. Configure signing in `gen/android/app/build.gradle`:
   ```gradle
   android {
     signingConfigs {
       release {
         storeFile file("/path/to/hippo-release.keystore")
         storePassword "your-password"
         keyAlias "hippo"
         keyPassword "your-password"
       }
     }
   }
   ```

3. Build AAB (required for Play Store):
   ```bash
   cd hippo-tauri
   cargo tauri android build --release --target aab
   ```

4. Upload to Google Play Console

## Available Make Commands

```bash
make mobile-help            # Show detailed help
make mobile-init            # Initialize mobile targets
make mobile-android         # Run Android in dev mode
make mobile-android-build   # Build Android release
make mobile-ios             # Run iOS in dev mode (macOS)
make mobile-ios-build       # Build iOS release (macOS)
make mobile-clean           # Clean mobile build artifacts
```

## npm Scripts

```bash
cd hippo-tauri/ui

npm run mobile:init         # Initialize mobile
npm run tauri:android       # Android dev
npm run tauri:android:build # Android release
npm run tauri:ios           # iOS dev
npm run tauri:ios:build     # iOS release
```

## Troubleshooting

### iOS Issues

**Problem: "No Development Team Found"**
```bash
# Solution: Add your Team ID to tauri.conf.json
{
  "bundle": {
    "iOS": {
      "developmentTeam": "ABCDEF1234"
    }
  }
}
```

**Problem: Provisioning Profile Errors**
- Open project in Xcode
- Go to Signing & Capabilities
- Enable "Automatically manage signing"
- Select your team

**Problem: Simulator won't start**
```bash
# Reset simulator
xcrun simctl erase all
```

### Android Issues

**Problem: "ANDROID_HOME not set"**
```bash
# Add to your shell profile
export ANDROID_HOME=$HOME/Library/Android/sdk  # macOS
export ANDROID_HOME=$HOME/Android/Sdk          # Linux
```

**Problem: "NDK not found"**
```bash
# Install via sdkmanager
sdkmanager --install "ndk;26.1.10909125"
export NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125
```

**Problem: "No connected devices"**
```bash
# Check devices
adb devices

# Restart adb
adb kill-server
adb start-server
```

**Problem: Build fails with "SDK location not found"**
```bash
# Create local.properties in gen/android/
echo "sdk.dir=/path/to/android/sdk" > gen/android/local.properties
```

### General Issues

**Problem: Rust compilation errors**
```bash
# Update Rust
rustup update

# Clean build
cargo clean
cd hippo-tauri && cargo tauri android build
```

**Problem: UI not updating**
```bash
# Rebuild UI
cd hippo-tauri/ui
npm run build

# Then rebuild mobile
cd ..
cargo tauri android dev  # or ios dev
```

## Platform Differences

### File System Access

**Desktop**: Full file system access
**Mobile**: Sandboxed access to app-specific directories

Mobile apps can access:
- App's documents directory
- App's cache directory
- User-selected files via file picker
- Photos library (with permission)

### Storage Locations

**iOS**:
- Documents: `$APP_SUPPORT/Documents/`
- Database: `$APP_SUPPORT/hippo.db`
- Thumbnails: `$CACHE/thumbnails/`

**Android**:
- Documents: `/data/data/com.greplabs.hippo/files/`
- Database: `/data/data/com.greplabs.hippo/files/hippo.db`
- Thumbnails: `/data/data/com.greplabs.hippo/cache/thumbnails/`

### Permissions

Mobile apps must request permissions at runtime:
- File access (configured in tauri.conf.json)
- Photos library (iOS: Info.plist, Android: AndroidManifest.xml)
- Camera/Microphone (both platforms)

See configuration files:
- `Info.plist.additions` (iOS)
- `AndroidManifest.xml.additions` (Android)

## Next Steps

1. **Test on Real Devices**: Emulators don't perfectly replicate real hardware
2. **Optimize Performance**: Mobile devices have less RAM/CPU than desktops
3. **Add Mobile UI**: Consider touch-friendly UI improvements
4. **Handle Permissions**: Implement runtime permission requests
5. **Test Offline**: Ensure app works without network
6. **Beta Testing**: Use TestFlight (iOS) and internal testing (Android)

## Resources

- [Tauri v2 Mobile Docs](https://v2.tauri.app/develop/mobile/)
- [Android Developer Docs](https://developer.android.com/docs)
- [iOS Developer Docs](https://developer.apple.com/documentation/)
- [Rust Mobile Guide](https://rust-mobile.github.io/)

## Support

For issues specific to Hippo's mobile builds, open an issue on GitHub with:
- Platform (iOS/Android)
- OS version
- Error messages
- Steps to reproduce
