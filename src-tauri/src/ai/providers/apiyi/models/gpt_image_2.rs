use serde_json::{json, Value};

use crate::ai::error::AIError;
use crate::ai::GenerateRequest;

use super::super::adapter::{ApiyiModelAdapter, PreparedRequest};

pub struct GptImage2Adapter;

impl GptImage2Adapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GptImage2Adapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolution + aspect ratio → pixel size string for GPT Image 2
fn resolve_gpt_size(resolution: &str, aspect_ratio: &str) -> String {
    let sizes: &[(&str, &[(&str, &str)])] = &[
        ("1K", &[
            ("1:1", "1024x1024"),
            ("4:3", "1152x864"),
            ("3:4", "864x1152"),
            ("3:2", "1536x1024"),
            ("2:3", "1024x1536"),
            ("16:9", "1792x1008"),
            ("9:16", "1008x1792"),
            ("21:9", "1568x672"),
        ]),
        ("2K", &[
            ("1:1", "2048x2048"),
            ("4:3", "2304x1728"),
            ("3:4", "1728x2304"),
            ("3:2", "2496x1664"),
            ("2:3", "1664x2496"),
            ("16:9", "2048x1152"),
            ("9:16", "1152x2048"),
            ("21:9", "2912x1248"),
        ]),
        ("4K", &[
            ("1:1", "2880x2880"),
            ("4:3", "3264x2448"),
            ("3:4", "2448x3264"),
            ("3:2", "3504x2336"),
            ("2:3", "2336x3504"),
            ("16:9", "3840x2160"),
            ("9:16", "2160x3840"),
            ("21:9", "3808x1632"),
        ]),
    ];

    for (res, entries) in sizes {
        if *res == resolution {
            for (ratio, size) in *entries {
                if *ratio == aspect_ratio {
                    return size.to_string();
                }
            }
        }
    }

    "1024x1024".to_string()
}

fn truncate_for_log(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>()
}

impl ApiyiModelAdapter for GptImage2Adapter {
    fn model_aliases(&self) -> &'static [&'static str] {
        &["apiyi/gpt-image-2", "gpt-image-2"]
    }

    fn build_request(
        &self,
        request: &GenerateRequest,
        base_url: &str,
    ) -> Result<PreparedRequest, AIError> {
        let has_reference_images = request
            .reference_images
            .as_ref()
            .map(|images| !images.is_empty())
            .unwrap_or(false);

        let size = resolve_gpt_size(&request.size, &request.aspect_ratio);

        if has_reference_images {
            // GPT Image 2 edits: multipart/form-data
            // All images as image[] array fields
            let summary = format!(
                "model: apiyi/gpt-image-2, mode: edit, size: {}, prompt: {}",
                size,
                truncate_for_log(&request.prompt, 100)
            );

            Ok(PreparedRequest {
                endpoint: format!("{}/v1/images/edits", base_url),
                body: json!({ "__multipart__": true }),
                is_multipart: true,
                summary,
            })
        } else {
            let body = json!({
                "model": "gpt-image-2",
                "prompt": request.prompt,
                "n": 1,
                "size": size,
                "quality": "auto"
            });

            let summary = format!(
                "model: apiyi/gpt-image-2, mode: generate, size: {}, prompt: {}",
                size,
                truncate_for_log(&request.prompt, 100)
            );

            Ok(PreparedRequest {
                endpoint: format!("{}/v1/images/generations", base_url),
                body,
                is_multipart: false,
                summary,
            })
        }
    }

    fn extract_image_source(&self, response_body: &Value) -> Result<String, AIError> {
        // GPT Image 2 returns either:
        // { "data": [{ "b64_json": "..." }] }
        // or
        // { "data": [{ "url": "https://..." }] }
        let data = response_body
            .pointer("/data/0")
            .ok_or_else(|| AIError::Provider("GPT Image 2 response missing data".to_string()))?;

        // Prefer b64_json
        if let Some(b64) = data.get("b64_json").and_then(|v| v.as_str()).filter(|v| !v.trim().is_empty()) {
            return Ok(format!("data:image/png;base64,{}", b64));
        }

        // Fallback to URL
        if let Some(url) = data.get("url").and_then(|v| v.as_str()).filter(|v| !v.trim().is_empty()) {
            return Ok(url.to_string());
        }

        Err(AIError::Provider(format!(
            "GPT Image 2 response has no image data: {}",
            response_body
        )))
    }
}

inventory::submit! {
    crate::ai::providers::apiyi::models::RegisteredApiyiModel {
        build: || Box::new(GptImage2Adapter::new()),
    }
}
