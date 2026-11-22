//! Real local CLIP ViT-B/32 image embeddings via ONNX Runtime.
//!
//! Compiled ONLY under the `image-clip` cargo feature (which pulls the optional
//! `ort` dependency). With the feature off, this module does not exist and the
//! image path in [`crate::embeddings`] uses the deterministic placeholder.
//!
//! The pipeline mirrors OpenAI's CLIP preprocessing:
//!   1. resize so the shorter side is 224px (bicubic-ish),
//!   2. center-crop 224x224,
//!   3. scale to `[0, 1]`, normalize with CLIP mean/std,
//!   4. lay out as a `[1, 3, 224, 224]` CHW tensor,
//!   5. run the ONNX vision tower → 512-dim embedding, L2-normalized.
//!
//! The ONNX model is auto-downloaded and cached on first use (mirroring the
//! Qdrant manager's download pattern); nothing is bundled at build time.
//!
//! COMPILE-VERIFIED but NOT runtime-tested in this environment: downloading the
//! model and running ONNX inference require network + the ONNX Runtime shared
//! library at runtime, neither of which is exercised here. The exact ONNX IO
//! tensor names are read from the loaded session at runtime (not hardcoded), so
//! the code adapts to the model's actual signature.
#![cfg(feature = "image-clip")]

use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImageView};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use parking_lot::Mutex;
use tracing::{debug, info};

use crate::embeddings::IMAGE_EMBEDDING_DIM;
use crate::error::{HippoError, Result};

/// HuggingFace repo hosting the ONNX export of the CLIP ViT-B/32 *vision* tower.
/// It emits 512-dim image embeddings directly — the same model Qdrant's
/// `fastembed` ships, hence a natural match for Hippo's 512-dim `hippo_images`
/// collection (`IMAGE_EMBEDDING_DIM`).
const CLIP_MODEL_URL: &str =
    "https://huggingface.co/Qdrant/clip-ViT-B-32-vision/resolve/main/model.onnx";
/// Local cache filename for the downloaded model.
const CLIP_MODEL_FILENAME: &str = "clip-vit-b32-vision.onnx";

/// CLIP ViT-B/32 preprocessing constants (OpenAI CLIP). The published values
/// carry more digits than `f32` can represent exactly; we keep the canonical
/// numbers (rounded to the nearest `f32` at compile time) rather than lossily
/// truncating them, so silence the precision lint here.
const CLIP_INPUT: u32 = 224;
#[allow(clippy::excessive_precision)]
const CLIP_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
#[allow(clippy::excessive_precision)]
const CLIP_STD: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];

/// A loaded CLIP vision encoder. Inference requires `&mut Session`, so the
/// session is held behind a mutex and the encoder itself is shared read-only.
pub(crate) struct ClipImageEncoder {
    session: Mutex<Session>,
}

impl ClipImageEncoder {
    /// Load the CLIP vision ONNX model, downloading + caching it on first use.
    ///
    /// NOT runtime-tested here (needs network + ONNX Runtime lib).
    pub(crate) async fn load(cache_dir: &Path) -> Result<Self> {
        let model_path = ensure_model(cache_dir).await?;
        let session = Session::builder()
            .map_err(onnx_err)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(onnx_err)?
            .commit_from_file(&model_path)
            .map_err(onnx_err)?;
        info!("CLIP image encoder loaded from {}", model_path.display());
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    /// Produce a 512-dim, L2-normalized CLIP embedding for a single image.
    pub(crate) fn embed(&self, img: &DynamicImage) -> Result<Vec<f32>> {
        let input = preprocess(img);
        let tensor =
            Tensor::from_array((vec![1_i64, 3, CLIP_INPUT as i64, CLIP_INPUT as i64], input))
                .map_err(onnx_err)?;

        let mut session = self.session.lock();
        // Read the real IO names from the model so we don't hardcode a name that
        // a particular export might not use.
        let input_name = session.inputs[0].name.clone();
        let outputs = session
            .run(ort::inputs![input_name => tensor])
            .map_err(onnx_err)?;

        let (_shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(onnx_err)?;
        let mut v = data.to_vec();

        // The vision tower emits exactly IMAGE_EMBEDDING_DIM values for one
        // image. Guard anyway so a surprising export can never corrupt the
        // fixed-dim Qdrant collection (dimension mismatch on upsert).
        if v.len() != IMAGE_EMBEDDING_DIM {
            return Err(HippoError::Other(format!(
                "CLIP model produced {} dims, expected {IMAGE_EMBEDDING_DIM}",
                v.len()
            )));
        }
        l2_normalize(&mut v);
        Ok(v)
    }
}

fn onnx_err(e: ort::Error) -> HippoError {
    HippoError::Other(format!("ONNX Runtime error: {e}"))
}

/// Resize (shorter side → 224), center-crop 224x224, scale to `[0,1]`,
/// normalize with CLIP mean/std, and flatten to CHW (`R` plane, then `G`, `B`).
fn preprocess(img: &DynamicImage) -> Vec<f32> {
    let (w, h) = img.dimensions();
    let (nw, nh) = if w <= h {
        (
            CLIP_INPUT,
            ((CLIP_INPUT as f32 * h as f32 / w.max(1) as f32).round() as u32).max(CLIP_INPUT),
        )
    } else {
        (
            ((CLIP_INPUT as f32 * w as f32 / h.max(1) as f32).round() as u32).max(CLIP_INPUT),
            CLIP_INPUT,
        )
    };

    let resized = img.resize_exact(nw, nh, image::imageops::FilterType::CatmullRom);
    let x = (nw - CLIP_INPUT) / 2;
    let y = (nh - CLIP_INPUT) / 2;
    let cropped = resized.crop_imm(x, y, CLIP_INPUT, CLIP_INPUT).to_rgb8();

    let plane = (CLIP_INPUT * CLIP_INPUT) as usize;
    let mut data = vec![0f32; 3 * plane];
    for (i, px) in cropped.pixels().enumerate() {
        for c in 0..3 {
            let val = px[c] as f32 / 255.0;
            data[c * plane + i] = (val - CLIP_MEAN[c]) / CLIP_STD[c];
        }
    }
    data
}

fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Ensure the ONNX model is cached locally, downloading it once on first use.
///
/// Writes to a `.part` temp file and renames into place so a crash mid-download
/// never leaves a truncated model that would poison every future load.
async fn ensure_model(cache_dir: &Path) -> Result<PathBuf> {
    let model_dir = cache_dir.join("clip");
    let model_path = model_dir.join(CLIP_MODEL_FILENAME);
    if model_path.exists() {
        debug!("CLIP model already cached at {}", model_path.display());
        return Ok(model_path);
    }

    std::fs::create_dir_all(&model_dir)
        .map_err(|e| HippoError::Other(format!("Failed to create CLIP model dir: {e}")))?;

    info!("Downloading CLIP ViT-B/32 vision model (first use only)...");
    let client = reqwest::Client::new();
    let resp = client
        .get(CLIP_MODEL_URL)
        .send()
        .await
        .map_err(|e| HippoError::Other(format!("CLIP model download failed: {e}")))?;
    if !resp.status().is_success() {
        return Err(HippoError::Other(format!(
            "CLIP model download failed: HTTP {}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| HippoError::Other(format!("CLIP model download read failed: {e}")))?;

    let tmp = model_dir.join(format!("{CLIP_MODEL_FILENAME}.part"));
    std::fs::write(&tmp, &bytes)
        .map_err(|e| HippoError::Other(format!("Failed to write CLIP model: {e}")))?;
    std::fs::rename(&tmp, &model_path)
        .map_err(|e| HippoError::Other(format!("Failed to finalize CLIP model: {e}")))?;
    info!("CLIP model cached at {}", model_path.display());
    Ok(model_path)
}
