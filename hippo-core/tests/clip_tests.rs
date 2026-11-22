//! Offline, deterministic tests for the image embedding path.
//!
//! These exercise ONLY the feature-OFF / fallback behavior: the fixed
//! placeholder image "embedding". They never download a model or run ONNX
//! inference, so they pass in the default build (no `image-clip` feature) and
//! are safe on every CI runner including Windows.
//!
//! The real CLIP path (`--features image-clip`) requires network + the ONNX
//! Runtime library and is intentionally NOT covered here.

use std::path::PathBuf;

use hippo_core::embeddings::{Embedder, IMAGE_EMBEDDING_DIM};
use hippo_core::models::{Memory, MemoryKind, Source};
use hippo_core::HippoConfig;
use tempfile::TempDir;

/// Offline embedder config: no Ollama probe (so no network), hash fallback for
/// text, placeholder for images. CLIP opt-in left at its default (false).
fn offline_config(data_dir: PathBuf) -> HippoConfig {
    HippoConfig {
        data_dir,
        local_embeddings: false,
        ai_api_key: None,
        qdrant_url: "http://127.0.0.1:59321".to_string(),
        ..HippoConfig::default()
    }
}

async fn offline_embedder(tmp: &TempDir) -> Embedder {
    Embedder::new(&offline_config(tmp.path().to_path_buf()))
        .await
        .expect("init embedder")
}

/// Write a small deterministic gradient PNG and return its path.
fn write_test_image(dir: &std::path::Path, name: &str, w: u32, h: u32) -> PathBuf {
    let mut img = image::RgbImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]);
    }
    let path = dir.join(name);
    img.save(&path).expect("save test png");
    path
}

fn image_memory(path: PathBuf) -> Memory {
    Memory::new(
        path,
        Source::Local {
            root_path: PathBuf::from("/tmp"),
        },
        MemoryKind::Image {
            width: 32,
            height: 32,
            format: "png".to_string(),
        },
    )
}

fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

#[tokio::test]
async fn placeholder_image_embedding_is_correct_dim_and_l2_normalized() {
    let tmp = tempfile::tempdir().unwrap();
    let embedder = offline_embedder(&tmp).await;
    let img = write_test_image(tmp.path(), "grad.png", 32, 32);

    let emb = embedder
        .embed_memory(&image_memory(img))
        .await
        .expect("embed image");

    // Must match the hippo_images collection dimension exactly.
    assert_eq!(
        emb.len(),
        IMAGE_EMBEDDING_DIM,
        "image embedding must be {IMAGE_EMBEDDING_DIM}-dim"
    );
    // A real (non-blank) image yields a unit vector.
    let norm = l2_norm(&emb);
    assert!(
        (norm - 1.0).abs() < 1e-4,
        "placeholder embedding should be L2-normalized, got norm {norm}"
    );
}

#[tokio::test]
async fn placeholder_image_embedding_fills_every_dimension() {
    // Regression guard for the old dead-code bug where the tail dims (and ~86 of
    // the 256 downsampled pixels) were never written. With a non-trivial image,
    // a correctly-filled vector has no all-zero tail.
    let tmp = tempfile::tempdir().unwrap();
    let embedder = offline_embedder(&tmp).await;
    let img = write_test_image(tmp.path(), "grad.png", 64, 64);

    let emb = embedder
        .embed_memory(&image_memory(img))
        .await
        .expect("embed image");

    assert_eq!(emb.len(), IMAGE_EMBEDDING_DIM);
    // The last few dims (previously left unfilled by the buggy guard) must now
    // carry pixel-derived signal for a gradient image.
    let tail_nonzero = emb[IMAGE_EMBEDDING_DIM - 4..].iter().any(|&x| x != 0.0);
    assert!(
        tail_nonzero,
        "tail dimensions must be filled (dead-code bug regression)"
    );
}

#[tokio::test]
async fn placeholder_image_embedding_is_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let embedder = offline_embedder(&tmp).await;
    let img = write_test_image(tmp.path(), "grad.png", 48, 24);

    let a = embedder
        .embed_memory(&image_memory(img.clone()))
        .await
        .unwrap();
    let b = embedder.embed_memory(&image_memory(img)).await.unwrap();
    assert_eq!(a, b, "same image must produce identical embeddings");
}

#[tokio::test]
async fn unreadable_image_falls_back_without_error() {
    // Indexing must never hard-fail on an image it can't read: the embedder
    // returns a correctly-sized (zero) vector instead of erroring.
    let tmp = tempfile::tempdir().unwrap();
    let embedder = offline_embedder(&tmp).await;
    let missing = tmp.path().join("does-not-exist.png");

    let emb = embedder
        .embed_memory(&image_memory(missing))
        .await
        .expect("embed must not hard-fail on unreadable image");

    assert_eq!(emb.len(), IMAGE_EMBEDDING_DIM);
}
