use serde_json::{json, Value};

use crate::ai::error::AIError;
use crate::ai::GenerateRequest;

use super::super::adapter::{ApiyiModelAdapter, PreparedRequest};

const API_MODEL_NAME: &str = "seedream-5-0-260128";

pub struct Seedream5Adapter;

impl Seedream5Adapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Seedream5Adapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Seedream 5.0 supports both tier names ("2K", "3K") and pixel sizes ("2048x2048").
/// We use tier names directly since the API accepts them.
fn resolve_seedream_size(resolution: &str, _aspect_ratio: &str) -> String {
    match resolution {
        "3K" => "3K".to_string(),
        _ => "2K".to_string(),
    }
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn truncate_for_log(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>()
}

impl ApiyiModelAdapter for Seedream5Adapter {
    fn model_aliases(&self) -> &'static [&'static str] {
        &["apiyi/seedream-5", "seedream-5-0-260128", "seedream-5-0-lite-260128"]
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

        let size = resolve_seedream_size(&request.size, &request.aspect_ratio);

        // Seedream 5.0: both text-to-image and image-to-image use /v1/images/generations
        // Image-to-image passes reference images as URL array in JSON body (no multipart)
        let mut body = json!({
            "model": API_MODEL_NAME,
            "prompt": request.prompt,
            "size": size,
            "response_format": "b64_json",
            "output_format": "png",
            "watermark": false
        });

        if has_reference_images {
            let reference_images = request.reference_images.as_deref().unwrap_or(&[]);
            // Seedream accepts HTTP URLs and data URLs in the image array
            let valid_images: Vec<String> = reference_images
                .iter()
                .filter(|s| is_http_url(s) || s.starts_with("data:"))
                .cloned()
                .collect();

            if valid_images.len() != reference_images.len() {
                return Err(AIError::InvalidRequest(
                    "Seedream 5.0 reference images must be HTTP URLs or data URLs".to_string(),
                ));
            }

            body["image"] = json!(valid_images);
            body["sequential_image_generation"] = json!("disabled");
        }

        let mode_label = if has_reference_images { "edit" } else { "generate" };
        let summary = format!(
            "model: apiyi/seedream-5, mode: {}, size: {}, prompt: {}",
            mode_label,
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

    fn extract_image_source(&self, response_body: &Value) -> Result<String, AIError> {
        // Seedream 5.0 returns:
        // { "data": [{ "b64_json": "..." }] }  when response_format=b64_json
        // { "data": [{ "url": "https://..." }] } when response_format=url
        let data = response_body
            .pointer("/data/0")
            .ok_or_else(|| AIError::Provider("Seedream 5.0 response missing data".to_string()))?;

        // Prefer b64_json
        if let Some(b64) = data.get("b64_json").and_then(|v| v.as_str()).filter(|v| !v.trim().is_empty()) {
            return Ok(format!("data:image/png;base64,{}", b64));
        }

        // Fallback to URL
        if let Some(url) = data.get("url").and_then(|v| v.as_str()).filter(|v| !v.trim().is_empty()) {
            return Ok(url.to_string());
        }

        Err(AIError::Provider(format!(
            "Seedream 5.0 response has no image data: {}",
            truncate_for_log(&response_body.to_string(), 500)
        )))
    }
}

inventory::submit! {
    crate::ai::providers::apiyi::models::RegisteredApiyiModel {
        build: || Box::new(Seedream5Adapter::new()),
    }
}
