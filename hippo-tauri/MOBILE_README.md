# Hippo Mobile Apps - Quick Start

Native iOS and Android apps for Hippo built with Tauri 2.

## Quick Commands

```bash
# First time setup
make mobile-init

# Run on Android
make mobile-android

# Run on iOS (macOS only)
make mobile-ios

# Build for production
make mobile-android-build  # Android APK/AAB
make mobile-ios-build      # iOS IPA

# Get help
make mobile-help
```

## Requirements

### Android (all platforms)
- Java 17+
- Android Studio with SDK & NDK
- ANDROID_HOME environment variable

### iOS (macOS only)
- Xcode 13+
- Apple Developer account
- Update `developmentTeam` in tauri.conf.json

## Configuration Files

- **tauri.conf.json** - Main Tauri configuration with mobile settings
- **Info.plist.additions** - iOS permissions and metadata
- **AndroidManifest.xml.additions** - Android permissions
- **android-file-paths.xml** - Android FileProvider paths
- **capabilities/mobile.json** - Mobile-specific Tauri capabilities

## Key Changes

1. **App Identifier**: Changed to `com.greplabs.hippo`
2. **iOS**: Minimum iOS 13.0, requires development team
3. **Android**: Minimum SDK 24 (Android 7.0)
4. **Permissions**: File access, photos, camera, microphone
5. **File Scopes**: App data, documents, pictures, videos, audio

## Mobile vs Desktop Differences

### Storage
- **Desktop**: `~/.local/share/Hippo/` or `~/Library/Application Support/Hippo/`
- **iOS**: App sandbox (`$APP_SUPPORT/`)
- **Android**: `/data/data/com.greplabs.hippo/files/`

### File Access
- **Desktop**: Full file system access
- **Mobile**: Sandboxed, user must select files/folders via picker

### UI Considerations
- Touch targets should be 44pt minimum
- Consider gesture navigation
- Bottom navigation bar instead of sidebar on small screens
- Pull-to-refresh for re-indexing

## Documentation

- **MOBILE_SETUP.md** - Complete setup guide with troubleshooting
- **MOBILE_ICONS.md** - Icon generation instructions

## Build Outputs

### Android
- APK: `gen/android/app/build/outputs/apk/release/`
- AAB: `gen/android/app/build/outputs/bundle/release/`

### iOS
- IPA: `gen/ios/build/Release-iphoneos/`
- Via Xcode: Product > Archive

## Development Workflow

1. **Initialize**: `make mobile-init` (one time)
2. **Configure**: Update Team ID for iOS
3. **Develop**: `make mobile-android` or `make mobile-ios`
4. **Test**: On emulator and real device
5. **Build**: `make mobile-android-build` or `make mobile-ios-build`
6. **Distribute**: Upload to Play Store / App Store

## Troubleshooting

### Android: "ANDROID_HOME not set"
```bash
export ANDROID_HOME=$HOME/Library/Android/sdk  # macOS
export ANDROID_HOME=$HOME/Android/Sdk          # Linux
```

### iOS: "No Development Team"
Add your Team ID to `tauri.conf.json`:
```json
{
  "bundle": {
    "iOS": {
      "developmentTeam": "YOUR_TEAM_ID"
    }
  }
}
```

### Build fails
```bash
# Clean and rebuild
make mobile-clean
cargo clean
make mobile-init
make mobile-android  # or mobile-ios
```

## Resources

- [Tauri Mobile Docs](https://v2.tauri.app/develop/mobile/)
- [Android Prerequisites](https://v2.tauri.app/start/prerequisites/#android)
- [iOS Prerequisites](https://v2.tauri.app/start/prerequisites/#ios)

## Status

✅ Configuration complete
✅ Build scripts ready
⏳ Awaiting mobile target initialization (`make mobile-init`)
⏳ Requires developer accounts for distribution

## Next Steps

1. Run `make mobile-init` to generate mobile project files
2. Configure signing credentials
3. Generate production icons
4. Test on real devices
5. Submit to app stores
