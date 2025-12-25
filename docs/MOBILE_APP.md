# Hippo Mobile App Architecture

## Overview

The Hippo mobile app provides a companion experience for accessing your indexed files from iOS and Android devices.

---

## Architecture Options

### Option 1: React Native + Rust Core (Recommended)

```
┌─────────────────────────────────────────────────────────────┐
│                    React Native App                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   iOS UI    │  │ Android UI  │  │  Shared JS  │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
│                           │                                  │
│                    ┌──────┴──────┐                          │
│                    │   Bridge    │                          │
│                    └──────┬──────┘                          │
│                           │                                  │
│  ┌────────────────────────┴────────────────────────┐        │
│  │              hippo-mobile (Rust)                 │        │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐       │        │
│  │  │  Search  │  │  Sync    │  │  Cache   │       │        │
│  │  └──────────┘  └──────────┘  └──────────┘       │        │
│  └──────────────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

**Pros:**
- Share core Rust logic with desktop
- Native performance
- Single codebase for iOS/Android UI

**Cons:**
- Complex native module setup
- Larger app size

### Option 2: Swift/Kotlin Native Apps

```
┌──────────────────────┐    ┌──────────────────────┐
│      iOS App         │    │    Android App       │
│  ┌────────────────┐  │    │  ┌────────────────┐  │
│  │    SwiftUI     │  │    │  │ Jetpack Compose│  │
│  └────────────────┘  │    │  └────────────────┘  │
│  ┌────────────────┐  │    │  ┌────────────────┐  │
│  │ hippo-ios.xcf  │  │    │  │ hippo-android  │  │
│  │    (Rust)      │  │    │  │     (Rust)     │  │
│  └────────────────┘  │    │  └────────────────┘  │
└──────────────────────┘    └──────────────────────┘
```

**Pros:**
- Best native experience
- Smaller app size
- Platform-specific optimizations

**Cons:**
- Two separate UI codebases
- More development effort

### Option 3: Tauri Mobile (Experimental)

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Mobile App                          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   WebView UI                         │    │
│  │              (Same as desktop!)                      │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                 hippo-core (Rust)                    │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**Pros:**
- Reuse desktop UI and Rust core
- Single codebase
- Fastest development

**Cons:**
- Tauri mobile is still beta
- WebView performance limitations

---

## Recommended: React Native + Rust Core

### Project Structure

```
hippo-mobile/
├── ios/                      # iOS native code
│   ├── Hippo/
│   │   ├── AppDelegate.swift
│   │   └── HippoNative.swift # Rust bridge
│   └── Hippo.xcodeproj
│
├── android/                  # Android native code
│   ├── app/
│   │   └── src/main/
│   │       ├── java/.../HippoModule.kt
│   │       └── jniLibs/      # Rust .so files
│   └── build.gradle
│
├── src/                      # React Native (TypeScript)
│   ├── App.tsx
│   ├── screens/
│   │   ├── HomeScreen.tsx
│   │   ├── SearchScreen.tsx
│   │   ├── FileDetailScreen.tsx
│   │   └── SettingsScreen.tsx
│   ├── components/
│   │   ├── FileGrid.tsx
│   │   ├── FileCard.tsx
│   │   ├── SearchBar.tsx
│   │   └── TagPill.tsx
│   ├── hooks/
│   │   ├── useSearch.ts
│   │   ├── useSync.ts
│   │   └── useHippo.ts
│   └── native/
│       └── HippoNative.ts    # Native module interface
│
├── rust/                     # Shared Rust code
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── mobile.rs         # Mobile-specific APIs
│       └── sync.rs           # Desktop sync protocol
│
├── package.json
├── metro.config.js
└── README.md
```

### Rust Mobile Module

```rust
// hippo-mobile/rust/src/lib.rs

use uniffi;

#[uniffi::export]
pub struct HippoMobile {
    cache: MobileCache,
    sync: SyncClient,
}

#[uniffi::export]
impl HippoMobile {
    #[uniffi::constructor]
    pub fn new(data_dir: String) -> Self {
        // Initialize mobile Hippo
    }

    pub async fn search(&self, query: String) -> Vec<MemoryPreview> {
        // Search local cache or sync from desktop
    }

    pub async fn sync_with_desktop(&self, host: String) -> Result<()> {
        // Sync memories from desktop Hippo
    }

    pub fn get_thumbnail(&self, memory_id: String) -> Option<Vec<u8>> {
        // Get cached thumbnail
    }
}

#[derive(uniffi::Record)]
pub struct MemoryPreview {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub size: u64,
    pub thumbnail_url: Option<String>,
    pub tags: Vec<String>,
}
```

### React Native Interface

```typescript
// hippo-mobile/src/native/HippoNative.ts

import { NativeModules } from 'react-native';

interface HippoNative {
  search(query: string): Promise<MemoryPreview[]>;
  syncWithDesktop(host: string): Promise<void>;
  getThumbnail(memoryId: string): Promise<string | null>;
}

export const Hippo: HippoNative = NativeModules.HippoNative;

// Hook for easy usage
export function useHippoSearch(query: string) {
  const [results, setResults] = useState<MemoryPreview[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (query.length < 2) return;

    setLoading(true);
    Hippo.search(query)
      .then(setResults)
      .finally(() => setLoading(false));
  }, [query]);

  return { results, loading };
}
```

---

## Sync Protocol

### Desktop-Mobile Communication

```
Desktop (Hippo)              Mobile (Hippo Mobile)
      │                              │
      │◄──── Discovery (mDNS) ──────│
      │                              │
      │◄──── Auth (QR Code) ────────│
      │                              │
      │───── Sync Manifest ─────────►│
      │                              │
      │◄──── Request Files ─────────│
      │                              │
      │───── Stream Data ───────────►│
      │                              │
      │◄──── ACK ───────────────────│
```

### Sync Manifest

```json
{
  "version": 1,
  "lastSync": "2024-12-25T00:00:00Z",
  "totalMemories": 15892,
  "changes": [
    {
      "id": "uuid-1",
      "action": "upsert",
      "memory": { ... }
    },
    {
      "id": "uuid-2",
      "action": "delete"
    }
  ]
}
```

---

## Mobile Features

### MVP (v1.0)

| Feature | Priority | Description |
|---------|----------|-------------|
| Search | P0 | Search synced memories |
| Browse | P0 | Grid/list view of files |
| Thumbnails | P0 | View image previews |
| Sync | P0 | Pull from desktop |
| Tags | P1 | Filter by tags |
| Favorites | P1 | View starred files |

### Future (v2.0)

| Feature | Description |
|---------|-------------|
| Camera Roll Import | Index phone photos |
| Share Extension | Add files from other apps |
| Widget | Quick search widget |
| Offline Mode | Full local cache |
| AI Chat | On-device Ollama (Apple Silicon) |
| Cross-Device Sync | Multiple desktops |

---

## UI Screens

### 1. Home Screen

```
┌─────────────────────────────┐
│ ≡  Hippo           🔄 ⚙️    │
├─────────────────────────────┤
│ ┌─────────────────────────┐ │
│ │ 🔍 Search your files... │ │
│ └─────────────────────────┘ │
├─────────────────────────────┤
│ Recent                      │
│ ┌─────┐ ┌─────┐ ┌─────┐    │
│ │ 📷  │ │ 📄  │ │ 💻  │    │
│ │     │ │     │ │     │    │
│ └─────┘ └─────┘ └─────┘    │
│ ┌─────┐ ┌─────┐ ┌─────┐    │
│ │ 🎵  │ │ 📁  │ │ 📷  │    │
│ │     │ │     │ │     │    │
│ └─────┘ └─────┘ └─────┘    │
├─────────────────────────────┤
│ Tags: vacation work 2024   │
├─────────────────────────────┤
│  🏠    🔍    ⭐    👤       │
└─────────────────────────────┘
```

### 2. Search Screen

```
┌─────────────────────────────┐
│ ← Search                    │
├─────────────────────────────┤
│ ┌─────────────────────────┐ │
│ │ vacation photos    ✕    │ │
│ └─────────────────────────┘ │
│ [All] [Images] [Docs] [Code]│
├─────────────────────────────┤
│ 23 results                  │
│ ┌─────────────────────────┐ │
│ │ 📷 beach_sunset.jpg     │ │
│ │    2.4 MB • Dec 15      │ │
│ │    vacation, beach      │ │
│ └─────────────────────────┘ │
│ ┌─────────────────────────┐ │
│ │ 📷 family_photo.jpg     │ │
│ │    1.8 MB • Dec 16      │ │
│ │    vacation, family     │ │
│ └─────────────────────────┘ │
└─────────────────────────────┘
```

### 3. File Detail

```
┌─────────────────────────────┐
│ ←                    ⭐ ⋯   │
├─────────────────────────────┤
│ ┌─────────────────────────┐ │
│ │                         │ │
│ │    [Photo Preview]      │ │
│ │                         │ │
│ └─────────────────────────┘ │
├─────────────────────────────┤
│ beach_sunset.jpg            │
│ 📍 /Photos/2024/vacation    │
├─────────────────────────────┤
│ Size: 2.4 MB                │
│ Dimensions: 4032 × 3024     │
│ Created: Dec 15, 2024       │
│ Camera: iPhone 15 Pro       │
├─────────────────────────────┤
│ Tags:                       │
│ [vacation] [beach] [+]      │
├─────────────────────────────┤
│     [Open on Desktop]       │
└─────────────────────────────┘
```

---

## Development Roadmap

### Phase 1: Foundation (2 weeks)
- [ ] Set up React Native project
- [ ] Integrate Rust via uniffi
- [ ] Basic search UI
- [ ] Local SQLite cache

### Phase 2: Sync (2 weeks)
- [ ] Desktop sync server
- [ ] mDNS discovery
- [ ] QR code pairing
- [ ] Delta sync protocol

### Phase 3: UI Polish (2 weeks)
- [ ] Grid/list views
- [ ] Thumbnail loading
- [ ] Pull to refresh
- [ ] Dark mode

### Phase 4: Features (2 weeks)
- [ ] Tag filtering
- [ ] Favorites
- [ ] Search history
- [ ] Settings screen

### Phase 5: Release (1 week)
- [ ] App Store assets
- [ ] TestFlight beta
- [ ] Play Store beta
- [ ] Documentation

---

## Technical Requirements

### iOS
- iOS 15.0+
- Swift 5.8+
- Xcode 15+

### Android
- Android 8.0+ (API 26)
- Kotlin 1.9+
- NDK for Rust

### React Native
- React Native 0.73+
- TypeScript 5.0+
- Hermes engine

---

<p align="center">
  <sub>🦛 Hippo Mobile - Your files, everywhere</sub>
</p>
