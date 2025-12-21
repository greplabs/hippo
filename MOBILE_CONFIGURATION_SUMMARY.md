# Hippo Mobile Apps - Configuration Summary

This document summarizes all changes made to enable native iOS and Android builds for Hippo using Tauri 2.

## Branch

All changes are on the `feature/mobile-apps` branch.

## Files Modified

### 1. `/hippo-tauri/tauri.conf.json`
**Changes:**
- Updated `identifier` from `com.hippo.app` to `com.greplabs.hippo` (required for mobile)
- Added `bundle.iOS` section with minimum iOS 13.0 requirement
- Added `bundle.android` section with minimum SDK 24 (Android 7.0)
- Added `plugins.fs` configuration with scoped permissions for mobile file access

**Mobile-specific settings:**
```json
{
  "bundle": {
    "iOS": {
      "minimumSystemVersion": "13.0",
      "developmentTeam": ""  // To be filled by developer
    },
    "android": {
      "minSdkVersion": 24,
      "versionCode": 1
    }
  },
  "plugins": {
    "fs": {
      "scope": ["$APPDATA/*", "$DOCUMENT/*", "$PICTURE/*", "$VIDEO/*"]
    }
  }
}
```

### 2. `/hippo-tauri/Cargo.toml`
**Changes:**
- Added mobile-specific dependencies with conditional compilation
- Added `tauri-plugin-barcode-scanner` for iOS and Android (example mobile plugin)

**New sections:**
```toml
[target.'cfg(target_os = "android")'.dependencies]
tauri-plugin-barcode-scanner = "2"

[target.'cfg(target_os = "ios")'.dependencies]
tauri-plugin-barcode-scanner = "2"
```

### 3. `/hippo-tauri/ui/package.json`
**Changes:**
- Added npm scripts for mobile development
- Added `@tauri-apps/cli` as dev dependency

**New scripts:**
```json
{
  "tauri:android": "tauri android dev",
  "tauri:android:build": "tauri android build",
  "tauri:ios": "tauri ios dev",
  "tauri:ios:build": "tauri ios build",
  "mobile:init": "tauri android init && tauri ios init"
}
```

### 4. `/Makefile`
**Changes:**
- Added comprehensive mobile build targets
- Added mobile-help command with prerequisites and instructions

**New commands:**
- `make mobile-init` - Initialize mobile targets
- `make mobile-android` - Run Android in dev mode
- `make mobile-android-build` - Build Android release
- `make mobile-ios` - Run iOS in dev mode
- `make mobile-ios-build` - Build iOS release
- `make mobile-clean` - Clean mobile artifacts
- `make mobile-help` - Show detailed help

### 5. `/.gitignore`
**Changes:**
- Added mobile build artifact exclusions

**New entries:**
```
# Mobile build artifacts (Tauri)
hippo-tauri/gen/
**/gen/android/
**/gen/ios/
*.apk
*.aab
*.ipa
*.xcarchive
```

## Files Created

### Configuration Files

#### 1. `/hippo-tauri/Info.plist.additions`
iOS-specific configuration containing:
- App metadata (display name, description)
- Privacy permission descriptions (photos, camera, microphone, files)
- Supported file types (images, videos, documents)
- Background modes for file indexing
- Interface orientations (portrait and landscape)
- Launch screen configuration

**Usage:** After `make mobile-init`, manually merge these entries into `gen/ios/Hippo/Info.plist`

#### 2. `/hippo-tauri/AndroidManifest.xml.additions`
Android-specific configuration containing:
- Required permissions (storage, camera, microphone, network)
- Scoped storage configuration (Android 11+)
- FileProvider setup for file sharing
- Hardware acceleration settings
- Feature declarations (camera, microphone as optional)
- Large heap for handling many files

**Usage:** After `make mobile-init`, manually merge these into `gen/android/app/src/main/AndroidManifest.xml`

#### 3. `/hippo-tauri/android-file-paths.xml`
FileProvider paths configuration for Android:
- Internal storage paths (files, cache)
- External storage paths (SD card, downloads)
- Media storage paths

**Usage:** Copy to `gen/android/app/src/main/res/xml/file_paths.xml` after initialization

#### 4. `/hippo-tauri/capabilities/mobile.json`
Tauri mobile-specific permissions:
- Core window and app permissions
- Dialog permissions (file picker, alerts)
- Shell permissions (open files)
- Comprehensive file system permissions
- Scoped access to app directories

### Documentation Files

#### 5. `/hippo-tauri/MOBILE_SETUP.md` (Comprehensive Guide)
Complete setup and build guide including:
- Prerequisites for all platforms (Rust, Android SDK, Xcode)
- Step-by-step installation instructions
- Environment variable configuration
- iOS development workflow (simulator and device)
- Android development workflow (emulator and device)
- Production build instructions
- Code signing setup (keystores, certificates)
- Troubleshooting guide
- Platform-specific differences
- File system access patterns

#### 6. `/hippo-tauri/MOBILE_ICONS.md` (Icon Generation Guide)
Icon creation instructions:
- Required icon sizes for iOS (10+ different sizes)
- Required densities for Android (mdpi to xxxhdpi)
- Adaptive icon setup for modern Android
- ImageMagick commands for batch generation
- Tauri CLI icon generation
- Testing icons on devices

#### 7. `/hippo-tauri/MOBILE_README.md` (Quick Reference)
Quick start guide with:
- Common commands (cheat sheet)
- Prerequisites summary
- Configuration file overview
- Platform differences
- Build output locations
- Development workflow
- Common troubleshooting

#### 8. `/MOBILE_CONFIGURATION_SUMMARY.md` (This File)
Complete summary of all changes and new files.

### CI/CD

#### 9. `/.github/workflows/mobile-build.yml.disabled`
GitHub Actions workflow (disabled by default):
- Android build job (Ubuntu runner)
- iOS build job (macOS runner)
- Release automation
- Artifact uploads
- Code signing setup (commented, requires secrets)

**To enable:** Rename to `.yml` and configure repository secrets

## Required Secrets (for CI/CD)

If enabling the GitHub Actions workflow, configure these secrets:

### iOS
- `APPLE_DEVELOPER_ID` - Apple Developer account email
- `APPLE_TEAM_ID` - Your Team ID from Apple Developer
- `APPLE_CERTIFICATE` - Base64-encoded P12 certificate
- `APPLE_CERTIFICATE_PASSWORD` - Certificate password
- `PROVISIONING_PROFILE` - Base64-encoded provisioning profile

### Android
- `ANDROID_KEYSTORE` - Base64-encoded keystore file
- `ANDROID_KEYSTORE_PASSWORD` - Keystore password
- `ANDROID_KEY_ALIAS` - Key alias
- `ANDROID_KEY_PASSWORD` - Key password

## Prerequisites Summary

### Android Development (All Platforms)

1. **Java JDK 17+**
2. **Android Studio** with:
   - Android SDK (API 24+)
   - Android NDK (26.1.10909125 or later)
   - SDK Build Tools
   - Platform Tools
3. **Environment Variables:**
   ```bash
   export ANDROID_HOME=$HOME/Library/Android/sdk  # macOS
   export NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125
   ```
4. **Rust Targets:**
   ```bash
   rustup target add aarch64-linux-android
   rustup target add armv7-linux-androideabi
   ```

### iOS Development (macOS Only)

1. **Xcode 13.0+** with Command Line Tools
2. **CocoaPods:** `sudo gem install cocoapods`
3. **Apple Developer Account** (free for testing, $99/year for distribution)
4. **Rust Targets:**
   ```bash
   rustup target add aarch64-apple-ios
   rustup target add aarch64-apple-ios-sim
   ```
5. **Team ID:** Update in `tauri.conf.json`

## First Time Setup

### Step 1: Install Prerequisites
Follow platform-specific instructions in `MOBILE_SETUP.md`

### Step 2: Initialize Mobile Targets
```bash
make mobile-init
```

This creates:
- `hippo-tauri/gen/android/` - Android project
- `hippo-tauri/gen/ios/` - Xcode project

### Step 3: Configure Permissions
Manually merge platform-specific configuration files:

**iOS:**
```bash
# Edit gen/ios/Hippo/Info.plist
# Add entries from Info.plist.additions
```

**Android:**
```bash
# Edit gen/android/app/src/main/AndroidManifest.xml
# Add entries from AndroidManifest.xml.additions

# Copy file provider paths
cp android-file-paths.xml gen/android/app/src/main/res/xml/file_paths.xml
```

### Step 4: Configure Signing

**iOS:**
```bash
# Edit tauri.conf.json
{
  "bundle": {
    "iOS": {
      "developmentTeam": "YOUR_TEAM_ID_HERE"
    }
  }
}
```

**Android (for release builds):**
```bash
# Create keystore
keytool -genkey -v -keystore hippo-release.keystore \
  -alias hippo -keyalg RSA -keysize 2048 -validity 10000

# Configure in gen/android/app/build.gradle
```

### Step 5: Generate Icons
```bash
cd hippo-tauri
cargo tauri icon icons/icon.png
```

Or follow manual instructions in `MOBILE_ICONS.md`

### Step 6: Build and Run

**Android:**
```bash
# Development
make mobile-android

# Production
make mobile-android-build
```

**iOS:**
```bash
# Development
make mobile-ios

# Production
make mobile-ios-build
```

## Build Outputs

### Android
- **APK:** `hippo-tauri/gen/android/app/build/outputs/apk/release/app-release.apk`
- **AAB:** `hippo-tauri/gen/android/app/build/outputs/bundle/release/app-release.aab`

### iOS
- **IPA:** `hippo-tauri/gen/ios/build/Release-iphoneos/Hippo.ipa`
- **Archive:** Via Xcode (Product > Archive)

## Key Features

### Mobile-Specific Capabilities
✅ File system access (sandboxed)
✅ Photo library integration
✅ Camera access (future feature)
✅ Native file picker
✅ Background processing
✅ Native sharing

### Cross-Platform Support
✅ Shared Rust core (`hippo-core`)
✅ Shared UI (`hippo-tauri/ui`)
✅ Platform-specific permissions
✅ Adaptive storage locations

### Development Features
✅ Hot reload on mobile
✅ Dev tools integration
✅ Native debugging (Xcode/Android Studio)
✅ Fast iteration cycle

## Platform Differences

| Feature | Desktop | iOS | Android |
|---------|---------|-----|---------|
| File Access | Full | Sandboxed | Sandboxed |
| DB Location | `~/.local/share/` | `$APP_SUPPORT/` | `/data/data/` |
| Permissions | None | Runtime | Runtime |
| Distribution | Direct download | App Store | Play Store |
| Min Version | Any | iOS 13+ | Android 7+ |

## Testing Checklist

- [ ] Initialize mobile targets
- [ ] Configure signing (iOS Team ID, Android keystore)
- [ ] Test on iOS Simulator
- [ ] Test on Android Emulator
- [ ] Test on physical iOS device
- [ ] Test on physical Android device
- [ ] Verify file access permissions
- [ ] Verify photo library access
- [ ] Test search functionality
- [ ] Test indexing workflow
- [ ] Verify thumbnail generation
- [ ] Test offline functionality
- [ ] Build release APK/AAB
- [ ] Build release IPA
- [ ] Test production builds

## Next Steps

1. **Initialize:** Run `make mobile-init`
2. **Configure:** Set up signing credentials
3. **Test:** Run on simulators/emulators
4. **Refine:** Optimize UI for mobile (touch targets, gestures)
5. **Build:** Create release builds
6. **Distribute:** Submit to App Store / Play Store

## Mobile UI Considerations (Future Enhancements)

While the current UI is responsive, consider these mobile-specific improvements:

1. **Navigation**
   - Bottom tab bar instead of sidebar
   - Hamburger menu for settings
   - Gesture-based navigation

2. **Touch Interactions**
   - Minimum 44pt touch targets
   - Swipe gestures (delete, archive)
   - Pull-to-refresh for re-indexing
   - Long-press context menus

3. **Performance**
   - Virtual scrolling for large lists
   - Lazy loading images
   - Background sync
   - Offline-first architecture

4. **Mobile Features**
   - Share sheet integration
   - Quick Actions (3D Touch/Haptic)
   - Widgets (iOS/Android)
   - Notifications for index completion

## Resources

- [Tauri v2 Mobile Documentation](https://v2.tauri.app/develop/mobile/)
- [Android Developer Prerequisites](https://v2.tauri.app/start/prerequisites/#android)
- [iOS Developer Prerequisites](https://v2.tauri.app/start/prerequisites/#ios)
- [Tauri Mobile Guide](https://v2.tauri.app/guides/mobile/)

## Support

For Hippo-specific mobile issues:
1. Check `MOBILE_SETUP.md` troubleshooting section
2. Review Tauri mobile documentation
3. Open GitHub issue with platform details

## License

Mobile configuration follows the same license as Hippo project.

---

**Status:** ✅ Configuration complete, ready for initialization
**Last Updated:** 2025-12-20
**Branch:** feature/mobile-apps
