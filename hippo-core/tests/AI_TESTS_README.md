# AI Feature Tests - Documentation

This test suite provides comprehensive testing for all AI-powered features in Hippo without requiring actual Ollama or Claude API services to be running.

## Overview

The tests use **mock implementations** of AI clients that return predefined responses, allowing for:
- Fast test execution (no network calls)
- Deterministic results (no AI variability)
- No API keys required
- No external services needed
- Complete test coverage of AI features

## Test Architecture

### Mock Clients

#### `MockClaudeClient`
Simulates Claude API responses for:
- File analysis and tagging
- Document summarization
- Code analysis
- Image captioning
- RAG queries
- Organization suggestions

#### `MockOllamaClient`
Simulates Ollama local AI responses for:
- Document analysis
- Code analysis
- Image captioning (vision models)
- RAG query handling
- Chat conversations
- Embedding generation (768-dimensional vectors)

### Trait-Based Design

The `AiAnalyzer` trait provides a common interface that both real and mock implementations can use:

```rust
#[async_trait::async_trait]
pub trait AiAnalyzer: Send + Sync {
    async fn analyze_file(&self, memory: &Memory) -> Result<FileAnalysis>;
    async fn summarize_text(&self, content: &str, file_name: &str) -> Result<DocumentSummary>;
    async fn summarize_code(&self, code: &str, language: &str, file_name: &str) -> Result<CodeSummary>;
    async fn suggest_tags(&self, memory: &Memory) -> Result<Vec<TagSuggestion>>;
    async fn caption_image(&self, path: &std::path::Path) -> Result<String>;
    async fn rag_query(&self, query: &str, docs: Vec<(String, String, f32)>) -> Result<String>;
}
```

## Test Categories

### 1. File Analysis Tests
- `test_mock_claude_file_analysis` - Claude-based file analysis with tag suggestions
- `test_mock_ollama_document_analysis` - Ollama document analysis
- `test_mock_code_analysis` - Code file analysis with complexity detection

### 2. Tag Suggestion Tests
- `test_tag_suggestions_for_image` - AI-generated tags for images
- `test_tag_suggestions_confidence_filtering` - Confidence-based tag filtering

### 3. Image Captioning Tests
- `test_image_captioning_claude` - Image descriptions via Claude
- `test_image_captioning_ollama` - Image descriptions via Ollama vision models

### 4. RAG (Retrieval-Augmented Generation) Tests
- `test_rag_query_with_context` - Context-based question answering
- `test_rag_query_ollama` - Local RAG using Ollama

### 5. Natural Language Processing Tests
- `test_parse_search_query` - Converting natural language to structured queries
- `test_complex_query_parsing` - Multi-constraint query parsing

### 6. Organization Tests
- `test_organization_suggestions` - AI-suggested folder structures

### 7. Similar File Detection Tests
- `test_similar_file_detection` - Finding related files by type, tags, and metadata

### 8. Duplicate Detection Tests
- `test_exact_duplicate_detection` - Hash-based exact duplicates
- `test_similar_name_duplicate_detection` - Name-based similar files

### 9. Embedding Tests
- `test_mock_embeddings_generation` - Vector embedding generation
- `test_single_embedding` - Single-text embedding

### 10. Chat Tests
- `test_chat_conversation` - Multi-turn conversations

### 11. Integration Tests
- `test_unified_client_ollama_fallback` - Provider fallback handling
- `test_unified_client_provider_switching` - Runtime provider changes

### 12. Error Handling Tests
- `test_mock_ollama_unavailable` - Graceful degradation when services unavailable
- `test_empty_response_handling` - Default response handling

### 13. Performance Tests
- `test_concurrent_analysis` - Parallel file analysis
- `test_embedding_batch_performance` - Batch embedding generation

## Running the Tests

### Run all AI tests:
```bash
cd hippo-core
cargo test --test ai_tests
```

### Run specific test:
```bash
cargo test --test ai_tests test_mock_claude_file_analysis
```

### Run with output:
```bash
cargo test --test ai_tests -- --nocapture
```

### Run in parallel (default):
```bash
cargo test --test ai_tests -- --test-threads=4
```

## Customizing Mock Responses

### Adding Custom Responses

```rust
let mock_client = MockClaudeClient::with_responses(vec![
    json!({
        "tags": [
            {"name": "custom-tag", "confidence": 90, "reason": "Custom reason"}
        ],
        "description": "Custom description",
        "suggested_folder": "custom/folder"
    }).to_string()
]);

let analysis = mock_client.analyze_file(&memory).await.unwrap();
```

### Adding Responses Dynamically

```rust
let mock_client = MockClaudeClient::new();

mock_client.add_response(
    json!({
        "summary": "Dynamic response",
        "key_topics": ["topic1", "topic2"]
    }).to_string()
).await;
```

## Mock Response Format

### File Analysis Response
```json
{
  "tags": [
    {
      "name": "tag-name",
      "confidence": 85,
      "reason": "Explanation for tag"
    }
  ],
  "description": "File description",
  "suggested_folder": "path/to/suggested/folder"
}
```

### Document Summary Response
```json
{
  "summary": "Document summary text",
  "key_topics": ["topic1", "topic2", "topic3"],
  "document_type": "article|code|notes|report",
  "sentiment": "positive|negative|neutral",
  "complexity": "simple|moderate|complex"
}
```

### Code Summary Response
```json
{
  "summary": "Code summary",
  "purpose": "main|library|utility|test",
  "main_functionality": ["func1 - desc", "func2 - desc"],
  "dependencies": ["dep1", "dep2"],
  "complexity": "simple|moderate|complex",
  "patterns": ["pattern1", "pattern2"],
  "suggested_tags": ["tag1", "tag2"]
}
```

## Embedding Format

Mock embeddings return deterministic 768-dimensional vectors (matching nomic-embed-text):

```rust
// Each text gets a unique embedding based on its index
embedding[j] = (text_index + dimension_index) * 0.01
```

## Integration with Real AI Services

While these tests use mocks, the same trait-based design allows for easy integration testing with real services:

```rust
// For integration tests (requires actual services):
#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_real_ollama_integration() {
    let ollama = OllamaClient::new();

    // Skip if Ollama not available
    if !ollama.is_available().await {
        return;
    }

    // Test with real service
    let result = ollama.analyze_document("content", "file.txt").await;
    assert!(result.is_ok());
}
```

## Test Helpers

### `create_test_memory`
Creates a test Memory instance with specified properties:

```rust
let memory = create_test_memory(
    "/path/to/file.rs",
    MemoryKind::Code {
        language: "Rust".to_string(),
        lines: 250,
    },
    vec!["rust", "backend"]
);
```

## Best Practices

1. **Keep mocks simple** - Return minimal valid responses
2. **Test edge cases** - Empty responses, missing fields, invalid JSON
3. **Use deterministic data** - Avoid random values in tests
4. **Test async properly** - Use `#[tokio::test]` for async tests
5. **Verify all response fields** - Check structure, not just success
6. **Test error paths** - Verify graceful degradation
7. **Document complex mocks** - Explain non-obvious test setups

## Coverage

Current test coverage includes:
- ✅ File analysis (images, documents, code, videos)
- ✅ Tag suggestions with confidence scoring
- ✅ Image captioning (Claude and Ollama)
- ✅ Natural language query parsing
- ✅ RAG query handling
- ✅ Organization suggestions
- ✅ Similar file detection (heuristic-based)
- ✅ Duplicate detection (hash and name-based)
- ✅ Embedding generation
- ✅ Chat conversations
- ✅ Provider switching
- ✅ Error handling
- ✅ Batch processing
- ✅ Concurrent operations

## Future Enhancements

Potential additions:
- [ ] Mock image analysis with object detection
- [ ] Mock face detection
- [ ] Mock OCR extraction
- [ ] Mock video scene detection
- [ ] Mock entity extraction for documents
- [ ] Mock sentiment analysis
- [ ] Streaming response mocks
- [ ] Rate limiting simulation
- [ ] Network error simulation
- [ ] Partial response handling

## Contributing

When adding new AI features:

1. Add corresponding mock methods to `MockClaudeClient` and/or `MockOllamaClient`
2. Create test cases for happy path, edge cases, and errors
3. Document the expected response format
4. Update this README with the new test category

## Related Files

- `/hippo-core/src/ai/mod.rs` - Real AI client implementations
- `/hippo-core/src/ai/analysis.rs` - File analysis functions
- `/hippo-core/src/ollama/mod.rs` - Ollama client implementation
- `/hippo-tauri/src/main.rs` - Tauri AI commands (lines 492-2224)

## Troubleshooting

### Tests failing with "no such field"
- Check mock response JSON matches the expected struct fields
- Verify all required fields are present in mock responses

### Async tests timing out
- Ensure `#[tokio::test]` attribute is present
- Check for deadlocks in mock implementations
- Verify no actual network calls are being made

### Compilation errors
- Ensure all AI types are exported from `hippo_core::ai`
- Check that trait bounds match (`Send + Sync` for async traits)
- Verify `async-trait` crate is in dependencies

## License

Same as Hippo project (MIT/Apache-2.0)
