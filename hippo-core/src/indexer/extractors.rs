//! Metadata extractors for various file types

use crate::error::Result;
use crate::models::*;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Extract metadata from a file based on its type
pub fn extract_metadata(path: &Path, kind: &MemoryKind) -> Result<MemoryMetadata> {
    let mut metadata = MemoryMetadata {
        title: path.file_stem().and_then(|s| s.to_str()).map(String::from),
        ..Default::default()
    };

    match kind {
        MemoryKind::Image { .. } => {
            extract_image_metadata(path, &mut metadata)?;
        }
        MemoryKind::Document { format, .. } => {
            extract_document_metadata(path, format, &mut metadata)?;
        }
        MemoryKind::Code { language, .. } => {
            extract_code_metadata(path, language, &mut metadata)?;
        }
        MemoryKind::Audio { .. } => {
            extract_audio_metadata(path, &mut metadata)?;
        }
        _ => {}
    }

    Ok(metadata)
}

/// Extract EXIF and other metadata from images
fn extract_image_metadata(path: &Path, metadata: &mut MemoryMetadata) -> Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Try to read EXIF data
    if let Ok(exif_reader) = exif::Reader::new().read_from_container(&mut reader) {
        let mut exif_data = ExifData {
            camera_make: None,
            camera_model: None,
            lens: None,
            focal_length: None,
            aperture: None,
            iso: None,
            shutter_speed: None,
            taken_at: None,
            gps: None,
        };

        // Camera info
        if let Some(field) = exif_reader.get_field(exif::Tag::Make, exif::In::PRIMARY) {
            exif_data.camera_make = Some(field.display_value().to_string());
        }
        if let Some(field) = exif_reader.get_field(exif::Tag::Model, exif::In::PRIMARY) {
            exif_data.camera_model = Some(field.display_value().to_string());
        }

        // Lens
        if let Some(field) = exif_reader.get_field(exif::Tag::LensModel, exif::In::PRIMARY) {
            exif_data.lens = Some(field.display_value().to_string());
        }

        // Exposure settings
        if let Some(field) = exif_reader.get_field(exif::Tag::FocalLength, exif::In::PRIMARY) {
            if let exif::Value::Rational(ref v) = field.value {
                if let Some(r) = v.first() {
                    exif_data.focal_length = Some(r.num as f32 / r.denom as f32);
                }
            }
        }

        if let Some(field) = exif_reader.get_field(exif::Tag::FNumber, exif::In::PRIMARY) {
            if let exif::Value::Rational(ref v) = field.value {
                if let Some(r) = v.first() {
                    exif_data.aperture = Some(r.num as f32 / r.denom as f32);
                }
            }
        }

        if let Some(field) =
            exif_reader.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY)
        {
            if let exif::Value::Short(ref v) = field.value {
                exif_data.iso = v.first().map(|&v| v as u32);
            }
        }

        if let Some(field) = exif_reader.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY) {
            exif_data.shutter_speed = Some(field.display_value().to_string());
        }

        // Date taken
        if let Some(field) = exif_reader.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
            let date_str = field.display_value().to_string();
            // Parse "2024:07:15 14:30:00" format
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(
                date_str.trim_matches('"'),
                "%Y:%m:%d %H:%M:%S",
            ) {
                exif_data.taken_at =
                    Some(chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc));
            }
        }

        // GPS coordinates
        let lat = extract_gps_coord(
            &exif_reader,
            exif::Tag::GPSLatitude,
            exif::Tag::GPSLatitudeRef,
        );
        let lon = extract_gps_coord(
            &exif_reader,
            exif::Tag::GPSLongitude,
            exif::Tag::GPSLongitudeRef,
        );

        if let (Some(lat), Some(lon)) = (lat, lon) {
            exif_data.gps = Some(GeoLocation {
                latitude: lat,
                longitude: lon,
                altitude: None,
                place_name: None,
                city: None,
                country: None,
            });
            metadata.location = exif_data.gps.clone();
        }

        metadata.exif = Some(exif_data);
    }

    Ok(())
}

/// Extract GPS coordinate from EXIF
fn extract_gps_coord(exif: &exif::Exif, coord_tag: exif::Tag, ref_tag: exif::Tag) -> Option<f64> {
    let coord_field = exif.get_field(coord_tag, exif::In::PRIMARY)?;
    let ref_field = exif.get_field(ref_tag, exif::In::PRIMARY)?;

    if let exif::Value::Rational(ref rationals) = coord_field.value {
        if rationals.len() >= 3 {
            let degrees = rationals[0].num as f64 / rationals[0].denom as f64;
            let minutes = rationals[1].num as f64 / rationals[1].denom as f64;
            let seconds = rationals[2].num as f64 / rationals[2].denom as f64;

            let mut coord = degrees + minutes / 60.0 + seconds / 3600.0;

            // Check reference (N/S or E/W)
            let ref_str = ref_field.display_value().to_string();
            if ref_str.contains('S') || ref_str.contains('W') {
                coord = -coord;
            }

            return Some(coord);
        }
    }

    None
}

/// Extract text and metadata from documents
fn extract_document_metadata(
    path: &Path,
    format: &DocumentFormat,
    metadata: &mut MemoryMetadata,
) -> Result<()> {
    match format {
        DocumentFormat::PlainText | DocumentFormat::Markdown => {
            let content = std::fs::read_to_string(path)?;
            metadata.word_count = Some(content.split_whitespace().count() as u32);
            metadata.text_preview = Some(content.chars().take(500).collect());
        }
        DocumentFormat::Pdf => {
            // PDF extraction would use pdf-extract crate
            // For now, just note it's a PDF
        }
        _ => {}
    }

    Ok(())
}

/// Extract metadata from code files
fn extract_code_metadata(path: &Path, language: &str, metadata: &mut MemoryMetadata) -> Result<()> {
    let content = std::fs::read_to_string(path)?;

    let mut code_info = CodeInfo {
        language: language.to_string(),
        lines_of_code: content.lines().count() as u32,
        imports: Vec::new(),
        exports: Vec::new(),
        functions: Vec::new(),
        dependencies: Vec::new(),
    };

    // Simple import extraction (language-specific parsing would be better)
    for line in content.lines() {
        let trimmed = line.trim();

        // Rust
        if trimmed.starts_with("use ") {
            if let Some(import) = trimmed
                .strip_prefix("use ")
                .and_then(|s| s.strip_suffix(';'))
            {
                code_info.imports.push(import.to_string());
            }
        }
        // Python, JavaScript/TypeScript imports
        else if trimmed.starts_with("import ")
            || trimmed.starts_with("from ")
            || trimmed.starts_with("require(")
        {
            code_info.imports.push(trimmed.to_string());
        }

        // Simple function detection
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            if let Some(name) = extract_function_name(trimmed, "fn ") {
                code_info.functions.push(FunctionInfo {
                    name,
                    line_start: 0, // Would need proper parsing
                    line_end: 0,
                    is_public: trimmed.starts_with("pub "),
                    doc_comment: None,
                });
            }
        } else if trimmed.starts_with("def ") {
            if let Some(name) = extract_function_name(trimmed, "def ") {
                code_info.functions.push(FunctionInfo {
                    name,
                    line_start: 0,
                    line_end: 0,
                    is_public: true,
                    doc_comment: None,
                });
            }
        } else if trimmed.starts_with("function ") || trimmed.contains("=> {") {
            if let Some(name) = extract_function_name(trimmed, "function ") {
                code_info.functions.push(FunctionInfo {
                    name,
                    line_start: 0,
                    line_end: 0,
                    is_public: true,
                    doc_comment: None,
                });
            }
        }
    }

    metadata.code_info = Some(code_info);
    metadata.text_preview = Some(content.chars().take(500).collect());

    Ok(())
}

fn extract_function_name(line: &str, prefix: &str) -> Option<String> {
    let after_prefix = line.split(prefix).nth(1)?;
    let name: String = after_prefix
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Extract metadata from audio files
fn extract_audio_metadata(_path: &Path, _metadata: &mut MemoryMetadata) -> Result<()> {
    // Would use symphonia for proper audio metadata extraction
    // For now, just set the title from filename
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name_extraction() {
        assert_eq!(
            extract_function_name("fn hello_world() {", "fn "),
            Some("hello_world".into())
        );
        assert_eq!(
            extract_function_name("pub fn process_data(x: i32)", "fn "),
            Some("process_data".into())
        );
        assert_eq!(
            extract_function_name("def calculate_sum(a, b):", "def "),
            Some("calculate_sum".into())
        );
    }
}
