//! AI Tagging Benchmark
//!
//! Compares response times of different Ollama models for tag generation.
//! Run with: cargo bench --bench ai_tagging_benchmark

use std::time::{Duration, Instant};

const TEST_PROMPTS: &[&str] = &[
    "Suggest 3-5 tags for image file: 'vacation_beach_2024.jpg'. Tags only, comma-separated:",
    "Suggest 3-5 tags for code file: 'auth_service.rs'. Tags only, comma-separated:",
    "Suggest 3-5 tags for document: 'quarterly_report_q4.pdf'. Tags only, comma-separated:",
    "Suggest 3-5 tags for video: 'tutorial_react_hooks.mp4'. Tags only, comma-separated:",
    "Suggest 3-5 tags for screenshot: 'Screenshot 2024-12-28.png'. Tags only, comma-separated:",
];

const MODELS: &[&str] = &[
    "qwen2:0.5b",    // Ultra-fast (352MB)
    "llama3.2:1b",   // Fast (1.3GB)
    "gemma2:2b",     // Balanced (1.6GB)
];

#[tokio::main]
async fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║          AI Tagging Model Benchmark                        ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Check if Ollama is available
    let client = reqwest::Client::new();
    match client.get("http://localhost:11434").send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("✓ Ollama is running\n");
        }
        _ => {
            println!("✗ Ollama is not running. Start it with: ollama serve");
            return;
        }
    }

    // Get available models
    let available_models = get_available_models(&client).await;
    println!("Available models: {:?}\n", available_models);

    println!("Running benchmarks with {} test prompts...\n", TEST_PROMPTS.len());
    println!("{:<15} {:>12} {:>12} {:>12} {:>12}", "Model", "Avg (ms)", "Min (ms)", "Max (ms)", "Status");
    println!("{}", "-".repeat(65));

    for model in MODELS {
        if !available_models.contains(&model.to_string()) {
            println!("{:<15} {:>12} {:>12} {:>12} {:>12}", model, "-", "-", "-", "Not installed");
            continue;
        }

        let results = benchmark_model(&client, model).await;

        if results.is_empty() {
            println!("{:<15} {:>12} {:>12} {:>12} {:>12}", model, "-", "-", "-", "Failed");
            continue;
        }

        let avg = results.iter().sum::<u128>() / results.len() as u128;
        let min = *results.iter().min().unwrap();
        let max = *results.iter().max().unwrap();

        println!("{:<15} {:>12} {:>12} {:>12} {:>12}",
            model, avg, min, max, "✓");
    }

    println!("\n{}", "=".repeat(65));
    println!("Benchmark complete!\n");

    // Recommendations
    println!("Recommendations:");
    println!("  • For auto-tagging: Use qwen2:0.5b (fastest, ~1-2s)");
    println!("  • For quality tags: Use gemma2:2b (better quality, ~3-5s)");
    println!("  • For batch processing: Use qwen2:0.5b with parallel requests");
}

async fn get_available_models(client: &reqwest::Client) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct ModelsResponse {
        models: Vec<ModelInfo>,
    }
    #[derive(serde::Deserialize)]
    struct ModelInfo {
        name: String,
    }

    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(resp) => {
            if let Ok(models) = resp.json::<ModelsResponse>().await {
                models.models.into_iter().map(|m| m.name).collect()
            } else {
                vec![]
            }
        }
        Err(_) => vec![],
    }
}

async fn benchmark_model(client: &reqwest::Client, model: &str) -> Vec<u128> {
    let mut results = Vec::new();

    for prompt in TEST_PROMPTS {
        let start = Instant::now();

        let request = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.5,
                "num_predict": 100
            }
        });

        match client
            .post("http://localhost:11434/api/generate")
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let elapsed = start.elapsed().as_millis();
                results.push(elapsed);
            }
            _ => {
                // Skip failed requests
            }
        }
    }

    results
}
