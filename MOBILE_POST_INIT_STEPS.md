# Post-Initialization Configuration Steps

After running `make mobile-init`, you need to manually merge some platform-specific configurations. This guide shows you exactly what to do.

## Overview

The `make mobile-init` command creates the mobile project files in:
- `hippo-tauri/gen/android/` - Android project
- `hippo-tauri/gen/ios/` - iOS/Xcode project

However, some permissions and configurations must be manually merged to avoid overwriting Tauri's auto-generated files.

## Step 1: iOS Configuration

### 1.1 Update Info.plist

**File to edit:** `hippo-tauri/gen/ios/Hippo/Info.plist`

**Reference file:** `hippo-tauri/Info.plist.additions`

**Method 1: Using Xcode (Recommended)**

1. Open the project:
   ```bash
   open hippo-tauri/gen/ios/Hippo.xcodeproj
   ```

2. Select "Hippo" target in left sidebar

3. Go to "Info" tab

4. Add the following keys (click + button):

   | Key | Type | Value |
   |-----|------|-------|
   | Privacy - Photo Library Usage Description | String | Hippo needs access to your photos to index and organize your image files. |
   | Privacy - Photo Library Add Usage Description | String | Hippo may save thumbnails and processed images to your photo library. |
   | Privacy - Camera Usage Description | String | Hippo can use your camera to capture and organize new photos. |
   | Privacy - Microphone Usage Description | String | Hippo needs microphone access to record and organize audio files. |
   | Privacy - Documents Folder Usage Description | String | Hippo needs access to your documents to index and organize files. |

5. Save (⌘S)

**Method 2: Using Terminal/Editor**

```bash
# Open in default editor
open hippo-tauri/gen/ios/Hippo/Info.plist

# Or use VS Code
code hippo-tauri/gen/ios/Hippo/Info.plist
```

Find the `<dict>` tag and add before `</dict>`:

```xml
<!-- Privacy Permissions -->
<key>NSPhotoLibraryUsageDescription</key>
<string>Hippo needs access to your photos to index and organize your image files.</string>

<key>NSPhotoLibraryAddUsageDescription</key>
<string>Hippo may save thumbnails and processed images to your photo library.</string>

<key>NSCameraUsageDescription</key>
<string>Hippo can use your camera to capture and organize new photos.</string>

<key>NSMicrophoneUsageDescription</key>
<string>Hippo needs microphone access to record and organize audio files.</string>

<key>NSDocumentsFolderUsageDescription</key>
<string>Hippo needs access to your documents to index and organize files.</string>

<!-- Background Modes -->
<key>UIBackgroundModes</key>
<array>
    <string>fetch</string>
    <string>processing</string>
</array>
```

**Complete reference:** See `hippo-tauri/Info.plist.additions` for all available keys

### 1.2 Configure Signing

**File to edit:** `hippo-tauri/tauri.conf.json`

1. Find your Team ID:
   - Open Xcode
   - Preferences (⌘,) > Accounts
   - Select your Apple ID
   - Click on your team
   - Copy the Team ID (looks like: ABCD123456)

2. Edit `hippo-tauri/tauri.conf.json`:
   ```json
   {
     "bundle": {
       "iOS": {
         "minimumSystemVersion": "13.0",
         "developmentTeam": "ABCD123456"  // ← Replace with your Team ID
       }
     }
   }
   ```

3. Alternatively, enable automatic signing in Xcode:
   - Open `hippo-tauri/gen/ios/Hippo.xcodeproj`
   - Select "Hippo" target
   - Go to "Signing & Capabilities" tab
   - Check "Automatically manage signing"
   - Select your team from dropdown

## Step 2: Android Configuration

### 2.1 Update AndroidManifest.xml

**File to edit:** `hippo-tauri/gen/android/app/src/main/AndroidManifest.xml`

**Reference file:** `hippo-tauri/AndroidManifest.xml.additions`

**Using Terminal/Editor:**

```bash
# Open in VS Code or your preferred editor
code hippo-tauri/gen/android/app/src/main/AndroidManifest.xml
```

Add these permissions inside the `<manifest>` tag (before `<application>`):

```xml
<!-- File and Media Permissions -->
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
<uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE"
                 android:maxSdkVersion="28" />
<uses-permission android:name="android.permission.READ_MEDIA_IMAGES" />
<uses-permission android:name="android.permission.READ_MEDIA_VIDEO" />
<uses-permission android:name="android.permission.READ_MEDIA_AUDIO" />
<uses-permission android:name="android.permission.CAMERA" />
<uses-permission android:name="android.permission.RECORD_AUDIO" />

<!-- Optional Features -->
<uses-feature android:name="android.hardware.camera" android:required="false" />
<uses-feature android:name="android.hardware.microphone" android:required="false" />
```

Inside the `<application>` tag, add:

```xml
<!-- FileProvider for file sharing -->
<provider
    android:name="androidx.core.content.FileProvider"
    android:authorities="${applicationId}.fileprovider"
    android:exported="false"
    android:grantUriPermissions="true">
    <meta-data
        android:name="android.support.FILE_PROVIDER_PATHS"
        android:resource="@xml/file_paths" />
</provider>
```

### 2.2 Add FileProvider Configuration

**Create directory:**
```bash
mkdir -p hippo-tauri/gen/android/app/src/main/res/xml
```

**Copy file:**
```bash
cp hippo-tauri/android-file-paths.xml \
   hippo-tauri/gen/android/app/src/main/res/xml/file_paths.xml
```

**Verify:**
```bash
cat hippo-tauri/gen/android/app/src/main/res/xml/file_paths.xml
```

Should contain:
```xml
<?xml version="1.0" encoding="utf-8"?>
<paths xmlns:android="http://schemas.android.com/apk/res/android">
    <files-path name="files" path="." />
    <cache-path name="cache" path="." />
    <external-path name="external" path="." />
    <external-files-path name="external_files" path="." />
    <external-cache-path name="external_cache" path="." />
    <external-media-path name="external_media" path="." />
</paths>
```

## Step 3: Generate Icons (Optional but Recommended)

### Using Tauri CLI (Automatic)

```bash
cd hippo-tauri
cargo tauri icon icons/icon.png
```

This generates all required sizes for iOS and Android.

### Manual Generation

See `MOBILE_ICONS.md` for detailed instructions.

## Step 4: Verify Configuration

### iOS Verification

1. Open project in Xcode:
   ```bash
   open hippo-tauri/gen/ios/Hippo.xcodeproj
   ```

2. Check Info.plist:
   - Select Hippo target
   - Go to Info tab
   - Verify privacy descriptions are present

3. Check Signing:
   - Go to Signing & Capabilities tab
   - Verify team is selected
   - Should show "Provisioning Profile: Xcode Managed Profile"

4. Build in Xcode (⌘B) - should succeed

### Android Verification

1. Check AndroidManifest.xml:
   ```bash
   grep -A 2 "READ_EXTERNAL_STORAGE" \
     hippo-tauri/gen/android/app/src/main/AndroidManifest.xml
   ```

   Should show the permission.

2. Check FileProvider:
   ```bash
   grep -A 5 "FileProvider" \
     hippo-tauri/gen/android/app/src/main/AndroidManifest.xml
   ```

   Should show the provider configuration.

3. Check file_paths.xml exists:
   ```bash
   ls -l hippo-tauri/gen/android/app/src/main/res/xml/file_paths.xml
   ```

4. Build with Gradle:
   ```bash
   cd hippo-tauri/gen/android
   ./gradlew assembleDebug
   ```

   Should succeed.

## Step 5: Test Build

### Test Android

```bash
# Ensure emulator is running or device is connected
adb devices

# Build and run
make mobile-android
```

**Expected behavior:**
- Gradle builds without errors
- App installs on device/emulator
- App launches and shows Hippo UI
- No permission errors in logcat

### Test iOS

```bash
# Build and run on simulator
make mobile-ios
```

**Expected behavior:**
- Xcode builds without errors
- Simulator launches
- App installs and opens
- Hippo UI appears

## Common Issues and Fixes

### iOS: "Missing required key: NSPhotoLibraryUsageDescription"

**Cause:** Privacy descriptions not added to Info.plist

**Fix:** Complete Step 1.1 above

### iOS: "No Development Team"

**Cause:** Team ID not configured

**Fix:** Complete Step 1.2 above

### Android: "Permission denied" errors

**Cause:** Permissions not added to AndroidManifest.xml

**Fix:** Complete Step 2.1 above

### Android: "FileProvider not found"

**Cause:** FileProvider configuration missing or file_paths.xml not created

**Fix:** Complete Step 2.2 above

### Both: "Icons missing" warnings

**Cause:** App icons not generated

**Fix:** Complete Step 3 above

## Automation Script (Optional)

Save this as `mobile-post-init.sh` in project root:

```bash
#!/bin/bash
set -e

echo "Running post-initialization configuration..."

# Check if gen directories exist
if [ ! -d "hippo-tauri/gen/ios" ] || [ ! -d "hippo-tauri/gen/android" ]; then
    echo "Error: Run 'make mobile-init' first"
    exit 1
fi

# Copy Android FileProvider config
echo "Copying Android FileProvider configuration..."
mkdir -p hippo-tauri/gen/android/app/src/main/res/xml
cp hippo-tauri/android-file-paths.xml \
   hippo-tauri/gen/android/app/src/main/res/xml/file_paths.xml

echo "Done! Now:"
echo "1. Add iOS privacy descriptions to gen/ios/Hippo/Info.plist"
echo "2. Add Android permissions to gen/android/app/src/main/AndroidManifest.xml"
echo "3. Update iOS Team ID in tauri.conf.json"
echo ""
echo "See MOBILE_POST_INIT_STEPS.md for details"
```

Make it executable:
```bash
chmod +x mobile-post-init.sh
./mobile-post-init.sh
```

## Quick Checklist

After initialization, verify:

- [ ] iOS Info.plist has privacy descriptions
- [ ] iOS signing configured (Team ID set)
- [ ] Android AndroidManifest.xml has permissions
- [ ] Android FileProvider configured
- [ ] Android file_paths.xml exists
- [ ] Icons generated (optional)
- [ ] iOS test build succeeds
- [ ] Android test build succeeds
- [ ] App launches on iOS simulator
- [ ] App launches on Android emulator

## Next Steps

Once configuration is complete:

1. Test on real devices (not just emulators)
2. Test file access permissions
3. Test photo library access
4. Verify database creation
5. Test search and indexing
6. Configure release signing
7. Create production builds

## Reference Files

- `Info.plist.additions` - Complete iOS configuration reference
- `AndroidManifest.xml.additions` - Complete Android configuration reference
- `android-file-paths.xml` - FileProvider paths
- `MOBILE_SETUP.md` - Comprehensive setup guide
- `MOBILE_ICONS.md` - Icon generation guide

## Support

If you encounter issues:
1. Check the troubleshooting section in MOBILE_SETUP.md
2. Verify all steps in this guide are completed
3. Check Tauri docs: https://v2.tauri.app/develop/mobile/
4. Review platform-specific documentation

---

**Last Updated:** 2025-12-20
**Tauri Version:** 2.1
**Minimum iOS:** 13.0
**Minimum Android:** 7.0 (API 24)
