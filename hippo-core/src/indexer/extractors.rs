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
        MemoryKind::Video { .. } => {
            extract_video_metadata(path, &mut metadata)?;
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
            exposure_time: None,
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
            // Also store numeric value in seconds
            if let exif::Value::Rational(ref v) = field.value {
                if let Some(r) = v.first() {
                    exif_data.exposure_time = Some(r.num as f32 / r.denom as f32);
                }
            }
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
            // Extract altitude if available
            let altitude = extract_gps_altitude(&exif_reader);

            exif_data.gps = Some(GeoLocation {
                latitude: lat,
                longitude: lon,
                altitude,
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

/// Extract GPS altitude from EXIF
fn extract_gps_altitude(exif: &exif::Exif) -> Option<f64> {
    let altitude_field = exif.get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY)?;

    if let exif::Value::Rational(ref rationals) = altitude_field.value {
        if let Some(r) = rationals.first() {
            let mut altitude = r.num as f64 / r.denom as f64;

            // Check altitude reference (0 = above sea level, 1 = below sea level)
            if let Some(ref_field) = exif.get_field(exif::Tag::GPSAltitudeRef, exif::In::PRIMARY) {
                if let exif::Value::Byte(ref bytes) = ref_field.value {
                    if let Some(&1) = bytes.first() {
                        altitude = -altitude;
                    }
                }
            }

            return Some(altitude);
        }
    }

    None
}

/// Extract metadata from video files using ffprobe
fn extract_video_metadata(path: &Path, metadata: &mut MemoryMetadata) -> Result<()> {
    use std::process::Command;

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_name,codec_type,bit_rate,width,height,r_frame_rate,sample_rate,channels:format=bit_rate",
            "-of",
            "json",
            path.to_str().unwrap_or(""),
        ])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let mut video_meta = VideoMetadata {
                    codec: None,
                    bitrate: None,
                    framerate: None,
                    width: None,
                    height: None,
                    audio_codec: None,
                    audio_channels: None,
                    audio_sample_rate: None,
                };

                // Extract format-level bitrate
                if let Some(format) = json.get("format") {
                    if let Some(bitrate_str) = format.get("bit_rate").and_then(|v| v.as_str()) {
                        if let Ok(bitrate) = bitrate_str.parse::<u64>() {
                            video_meta.bitrate = Some(bitrate);
                        }
                    }
                }

                // Extract stream information
                if let Some(streams) = json.get("streams").and_then(|v| v.as_array()) {
                    for stream in streams {
                        let codec_type = stream
                            .get("codec_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        match codec_type {
                            "video" => {
                                // Video codec
                                if let Some(codec) = stream.get("codec_name").and_then(|v| v.as_str())
                                {
                                    video_meta.codec = Some(codec.to_string());
                                }

                                // Dimensions
                                if let Some(width) = stream.get("width").and_then(|v| v.as_u64()) {
                                    video_meta.width = Some(width as u32);
                                }
                                if let Some(height) = stream.get("height").and_then(|v| v.as_u64()) {
                                    video_meta.height = Some(height as u32);
                                }

                                // Frame rate (parse "30/1" or "24000/1001" format)
                                if let Some(fps_str) =
                                    stream.get("r_frame_rate").and_then(|v| v.as_str())
                                {
                                    if let Some((num, den)) = fps_str.split_once('/') {
                                        if let (Ok(n), Ok(d)) =
                                            (num.parse::<f32>(), den.parse::<f32>())
                                        {
                                            if d > 0.0 {
                                                video_meta.framerate = Some(n / d);
                                            }
                                        }
                                    }
                                }

                                // Stream-level bitrate (if format-level not available)
                                if video_meta.bitrate.is_none() {
                                    if let Some(bitrate_str) =
                                        stream.get("bit_rate").and_then(|v| v.as_str())
                                    {
                                        if let Ok(bitrate) = bitrate_str.parse::<u64>() {
                                            video_meta.bitrate = Some(bitrate);
                                        }
                                    }
                                }
                            }
                            "audio" => {
                                // Audio codec
                                if let Some(codec) = stream.get("codec_name").and_then(|v| v.as_str())
                                {
                                    video_meta.audio_codec = Some(codec.to_string());
                                }

                                // Sample rate
                                if let Some(sample_rate_str) =
                                    stream.get("sample_rate").and_then(|v| v.as_str())
                                {
                                    if let Ok(sample_rate) = sample_rate_str.parse::<u32>() {
                                        video_meta.audio_sample_rate = Some(sample_rate);
                                    }
                                }

                                // Channels
                                if let Some(channels) = stream.get("channels").and_then(|v| v.as_u64())
                                {
                                    video_meta.audio_channels = Some(channels as u32);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                metadata.video_metadata = Some(video_meta);
            }
        }
    }

    Ok(())
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

/// Extract metadata from audio files using symphonia
fn extract_audio_metadata(path: &Path, metadata: &mut MemoryMetadata) -> Result<()> {
    use symphonia::core::codecs::CODEC_TYPE_NULL;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    // Try to open the file
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint based on file extension
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    // Probe the media source
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    if let Ok(mut probed) = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
        let mut audio_meta = AudioMetadata {
            codec: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            artist: None,
            album: None,
            title: None,
            track_number: None,
            genre: None,
            year: None,
        };

        // Get the default track (usually the first audio track)
        if let Some(track) = probed
            .format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        {
            // Codec
            let codec_registry = symphonia::default::get_codecs();
            if let Some(codec_desc) = codec_registry.get_codec(track.codec_params.codec) {
                audio_meta.codec = Some(codec_desc.short_name.to_string());
            }

            // Sample rate
            if let Some(sample_rate) = track.codec_params.sample_rate {
                audio_meta.sample_rate = Some(sample_rate);
            }

            // Channels
            if let Some(channels) = track.codec_params.channels {
                audio_meta.channels = Some(channels.count() as u32);
            }

            // Bitrate (if available)
            if let Some(n_frames) = track.codec_params.n_frames {
                if let Some(sample_rate) = track.codec_params.sample_rate {
                    if let Some(_bits_per_sample) = track.codec_params.bits_per_sample {
                        if let Some(_channels) = track.codec_params.channels {
                            // Calculate approximate bitrate
                            let duration_secs = n_frames as f64 / sample_rate as f64;
                            if duration_secs > 0.0 {
                                let file_size = std::fs::metadata(path)
                                    .map(|m| m.len())
                                    .unwrap_or(0);
                                audio_meta.bitrate = Some((file_size * 8) / duration_secs as u64);
                            }
                        }
                    }
                }
            }
        }

        // Extract metadata tags (ID3, Vorbis comments, etc.)
        let metadata_rev = probed.format.metadata();

        // Try getting metadata from the format's metadata
        if let Some(metadata) = metadata_rev.current() {
            for tag in metadata.tags() {
                match tag.std_key {
                    Some(symphonia::core::meta::StandardTagKey::Artist) => {
                        audio_meta.artist = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Album) => {
                        audio_meta.album = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                        audio_meta.title = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::TrackNumber) => {
                        if let Ok(num) = tag.value.to_string().parse::<u32>() {
                            audio_meta.track_number = Some(num);
                        }
                    }
                    Some(symphonia::core::meta::StandardTagKey::Genre) => {
                        audio_meta.genre = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Date) => {
                        // Try to extract year from date
                        let date_str = tag.value.to_string();
                        if let Some(year_str) = date_str.split('-').next() {
                            if let Ok(year) = year_str.parse::<u32>() {
                                audio_meta.year = Some(year);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Also check probed metadata (some formats store metadata differently)
        if let Some(metadata) = probed.metadata.get() {
            if let Some(current) = metadata.current() {
                for tag in current.tags() {
                    // Fill in any missing fields
                    match tag.std_key {
                        Some(symphonia::core::meta::StandardTagKey::Artist) if audio_meta.artist.is_none() => {
                            audio_meta.artist = Some(tag.value.to_string());
                        }
                        Some(symphonia::core::meta::StandardTagKey::Album) if audio_meta.album.is_none() => {
                            audio_meta.album = Some(tag.value.to_string());
                        }
                        Some(symphonia::core::meta::StandardTagKey::TrackTitle) if audio_meta.title.is_none() => {
                            audio_meta.title = Some(tag.value.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Set title in metadata if found
        if let Some(title) = &audio_meta.title {
            metadata.title = Some(title.clone());
        }

        metadata.audio_metadata = Some(audio_meta);
    }

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
