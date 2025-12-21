# PDF and Document Thumbnail Implementation

## Overview
This document describes the implementation of PDF and Office document thumbnail generation added to the Hippo project on the `feature/pdf-previews` branch.

## Changes Made

### 1. Dependencies Added

#### Workspace-level (Cargo.toml)
- Added `pdfium-render = "0.8"` for PDF rendering capabilities

#### hippo-core (hippo-core/Cargo.toml)
- Added `pdfium-render = { workspace = true }`
- Added `zip = { workspace = true }` (for Office document extraction)

### 2. Thumbnail Module Updates (hippo-core/src/thumbnails/mod.rs)

#### New Imports
- Added `std::io::{Read, Seek}` for ZIP file operations

#### New Public Methods

##### `generate_pdf_thumbnail(&self, pdf_path: &Path) -> Result<PathBuf>`
Generates a 256x256 JPEG thumbnail from the first page of a PDF document.

**Features:**
- Smart caching (checks if thumbnail exists and is newer than source)
- Uses pdfium-render library for high-quality rendering
- Maintains aspect ratio (landscape/portrait detection)
- Generates consistent 256x256 thumbnails matching existing image/video thumbnails
- Graceful error handling with detailed error messages

**Technical Details:**
- Initializes pdfium library (tries local path first, falls back to system library)
- Loads PDF document and extracts first page
- Calculates appropriate render dimensions based on page aspect ratio
- Renders page to bitmap with proper scaling
- Converts bitmap to DynamicImage (RGBA8)
- Applies standard thumbnail resizing (256x256 max, maintaining aspect ratio)
- Saves as JPEG for consistency with other thumbnails

##### `generate_office_thumbnail(&self, doc_path: &Path) -> Result<PathBuf>`
Extracts embedded thumbnails from Office Open XML documents (.docx, .xlsx, .pptx).

**Features:**
- Smart caching (same as PDF)
- Extracts pre-rendered thumbnails from Office document ZIP archives
- Supports multiple thumbnail formats (JPEG, PNG, EMF, WMF)
- Consistent 256x256 JPEG output

**Technical Details:**
- Office documents are ZIP archives containing a `docProps/thumbnail.*` file
- Tries multiple known thumbnail paths in the ZIP structure
- Extracts thumbnail image data
- Loads image from memory and applies standard thumbnail resizing
- Saves as JPEG for consistency

#### New Helper Functions

##### `is_supported_pdf(path: &Path) -> bool`
Checks if a file is a PDF document by extension.

**Supported:**
- `.pdf` (case-insensitive)

##### `is_supported_office_document(path: &Path) -> bool`
Checks if a file is a supported Office document format.

**Supported:**
- `.docx` (Word documents)
- `.xlsx` (Excel spreadsheets)
- `.pptx` (PowerPoint presentations)

Note: Old Office formats (.doc, .xls, .ppt) are NOT supported as they use a different binary format.

#### New Tests
- `test_is_supported_pdf()` - Tests PDF file detection
- `test_is_supported_office_document()` - Tests Office document detection

### 3. Implementation Strategy

#### PDF Rendering
The implementation uses `pdfium-render` which provides:
- Cross-platform PDF rendering via Google's PDFium library
- High-quality rasterization
- Built-in support for complex PDF features (fonts, images, vectors)
- No external dependencies (pdftoppm, poppler) required

#### Office Document Thumbnails
Office Open XML documents (.docx, .xlsx, .pptx) are ZIP archives with a well-defined structure:
```
document.docx/
├── docProps/
│   └── thumbnail.jpeg  <- Embedded thumbnail
├── word/
│   └── document.xml
└── _rels/
```

The implementation extracts this pre-rendered thumbnail, avoiding the need to:
- Parse complex document formats
- Render document content
- Handle fonts and layouts

#### Fallback Strategy
- **PDF:** If pdfium fails to load, error is returned (no fallback currently)
- **Office:** If no embedded thumbnail exists, error is returned
- Both methods use the same caching and error handling patterns as images/videos

## Usage Example

```rust
use hippo_core::thumbnails::ThumbnailManager;
use std::path::Path;

let thumbnail_manager = ThumbnailManager::new()?;

// Generate PDF thumbnail
let pdf_path = Path::new("/path/to/document.pdf");
if is_supported_pdf(pdf_path) {
    let thumbnail_path = thumbnail_manager.generate_pdf_thumbnail(pdf_path)?;
    let thumbnail_data = thumbnail_manager.get_thumbnail_data(pdf_path)?;
}

// Extract Office document thumbnail
let doc_path = Path::new("/path/to/document.docx");
if is_supported_office_document(doc_path) {
    let thumbnail_path = thumbnail_manager.generate_office_thumbnail(doc_path)?;
    let thumbnail_data = thumbnail_manager.get_thumbnail_data(doc_path)?;
}
```

## Integration Points

To fully integrate PDF and document thumbnails into Hippo:

### 1. Update Indexer (hippo-core/src/indexer/mod.rs)
When indexing documents, generate thumbnails:

```rust
use crate::thumbnails::{is_supported_pdf, is_supported_office_document};

// In the indexing logic
if is_supported_pdf(&path) {
    if let Ok(thumb_path) = self.thumbnail_manager.generate_pdf_thumbnail(&path) {
        debug!("PDF thumbnail generated: {:?}", thumb_path);
    }
} else if is_supported_office_document(&path) {
    if let Ok(thumb_path) = self.thumbnail_manager.generate_office_thumbnail(&path) {
        debug!("Office thumbnail generated: {:?}", thumb_path);
    }
}
```

### 2. Update Tauri Commands (hippo-tauri/src/main.rs)
Add commands to retrieve document thumbnails (similar to existing image thumbnails).

### 3. Update UI (hippo-tauri/ui/dist/index.html)
Display document thumbnails in the grid view:
- PDF icon with first page preview
- Office document icons with embedded thumbnails
- Fallback to document type icons if thumbnail generation fails

## Performance Considerations

### PDF Rendering
- **First render:** May take 500ms-2s depending on PDF complexity
- **Cached access:** <1ms (disk cache) or microseconds (memory cache)
- **Memory:** Minimal, as pdfium streams the rendering

### Office Document Extraction
- **First extract:** 10-50ms (ZIP parsing + image extraction)
- **Cached access:** <1ms (disk cache) or microseconds (memory cache)
- **Memory:** Very low, as only thumbnail is extracted

### Caching Strategy
Both methods use the same two-tier caching as images/videos:
1. **Disk cache:** Persistent across restarts (~/.cache/Hippo/thumbnails/)
2. **Memory cache:** LRU cache (500 entries, 50MB max)
3. **Smart invalidation:** Regenerates if source file is newer than thumbnail

## Limitations

1. **PDF:**
   - Requires pdfium library to be available
   - Only renders first page (configurable if needed)
   - Password-protected PDFs not supported

2. **Office Documents:**
   - Only supports Office Open XML formats (.docx, .xlsx, .pptx)
   - Old binary formats (.doc, .xls, .ppt) not supported
   - Requires document to have embedded thumbnail (most do, but not guaranteed)
   - No fallback for documents without thumbnails

3. **Text Documents:**
   - Plain text files (.txt, .md) have no thumbnail support yet
   - Could be added by rendering first few lines as image (optional feature)

## Future Enhancements

1. **Text file previews:**
   - Render first 10-20 lines as an image
   - Syntax highlighting for code files
   - Use image crate to draw text on canvas

2. **Better PDF fallbacks:**
   - Try command-line tools (pdftoppm, pdftocairo) if pdfium fails
   - Support for password-protected PDFs

3. **Richer Office support:**
   - Generate thumbnails for documents without embedded ones
   - Use LibreOffice headless mode for rendering
   - Support for .odt, .ods, .odp (OpenDocument formats)

4. **Optimization:**
   - Batch PDF rendering for multiple documents
   - Async thumbnail generation
   - Progressive loading (low-quality preview, then high-quality)

## Testing

Run the tests to verify the implementation:

```bash
# Test hippo-core including new thumbnail tests
cargo test -p hippo-core thumbnails

# Build the entire project
cargo build

# Run with logging to see thumbnail generation
RUST_LOG=debug cargo run -p hippo-tauri
```

## Notes

- All thumbnails are consistently 256x256 JPEG files
- Aspect ratio is maintained (smaller dimension = 256px)
- SHA256 hash of file path is used for cache filenames
- Same error handling and logging patterns as existing thumbnail code
- No breaking changes to existing APIs
