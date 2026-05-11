use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::ai::error::AIError;
use crate::ai::GenerateRequest;

use super::super::adapter::{ApiyiModelAdapter, PreparedRequest};

const API_MODEL_NAME: &str = "gemini-3.1-flash-image-preview";

pub struct NanoBanana2Adapter;

impl NanoBanana2Adapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NanoBanana2Adapter {
    fn default() -> Self {
        Self::new()
    }
}

fn decode_file_url_path(value: &str) -> String {
    let raw = value.trim_start_matches("file://");
    let decoded = urlencoding::decode(raw)
        .map(|result| result.into_owned())
        .unwrap_or_else(|_| raw.to_string());
    let normalized = if decoded.starts_with('/')
        && decoded.len() > 2
        && decoded.as_bytes().get(2) == Some(&b':')
    {
        &decoded[1..]
    } else {
        &decoded
    };
    normalized.to_string()
}

fn source_to_base64(source: &str) -> Result<String, String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err("source is empty".to_string());
    }

    // Already a data URL
    if let Some((meta, payload)) = trimmed.split_once(',') {
        if meta.starts_with("data:") && meta.ends_with(";base64") && !payload.is_empty() {
            return Ok(payload.to_string());
        }
    }

    // Raw base64
    let likely_base64 = trimmed.len() > 256
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '=');
    if likely_base64 {
        return Ok(trimmed.to_string());
    }

    // File path
    let path = if trimmed.starts_with("file://") {
        PathBuf::from(decode_file_url_path(trimmed))
    } else {
        PathBuf::from(trimmed)
    };
    let bytes = std::fs::read(&path).map_err(|err| {
        format!(
            "failed to read path \"{}\": {}",
            path.to_string_lossy(),
            err
        )
    })?;
    Ok(STANDARD.encode(bytes))
}

fn infer_mime_type(source: &str) -> String {
    if source.contains("image/png") || source.to_lowercase().ends_with(".png") {
        "image/png".to_string()
    } else {
        "image/jpeg".to_string()
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

impl ApiyiModelAdapter for NanoBanana2Adapter {
    fn model_aliases(&self) -> &'static [&'static str] {
        &["apiyi/nano-banana-2", "nano-banana-2"]
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

        let thinking_level = request
            .extra_params
            .as_ref()
            .and_then(|params| params.get("thinking_level"))
            .and_then(|raw| raw.as_str())
            .map(|value| value.trim().to_lowercase())
            .filter(|value| value == "minimal" || value == "high")
            .unwrap_or_else(|| "minimal".to_string());

        let mut parts = vec![json!({ "text": request.prompt })];

        if has_reference_images {
            let reference_images = request.reference_images.as_deref().unwrap_or(&[]);
            for source in reference_images {
                if is_http_url(source) {
                    let mime = infer_mime_type(source);
                    parts.push(json!({
                        "fileData": {
                            "fileUri": source,
                            "mimeType": mime
                        }
                    }));
                } else {
                    let base64_data = source_to_base64(source).map_err(|err| {
                        AIError::InvalidRequest(format!(
                            "Failed to encode reference image for Nano Banana 2: {}",
                            err
                        ))
                    })?;
                    let mime = infer_mime_type(source);
                    parts.push(json!({
                        "inlineData": {
                            "data": base64_data,
                            "mimeType": mime
                        }
                    }));
                }
            }
        }

        let body = json!({
            "contents": [{
                "parts": parts
            }],
            "generationConfig": {
                "responseModalities": ["TEXT", "IMAGE"],
                "imageConfig": {
                    "imageSize": request.size,
                    "aspectRatio": request.aspect_ratio
                },
                "thinkingConfig": {
                    "thinkingLevel": thinking_level
                }
            }
        });

        let endpoint = format!(
            "{}/v1beta/models/{}:generateContent",
            base_url, API_MODEL_NAME
        );

        let mode_label = if has_reference_images { "edit" } else { "generate" };
        let summary = format!(
            "model: apiyi/nano-banana-2, mode: {}, size: {}, aspect_ratio: {}, thinking: {}, prompt: {}",
            mode_label,
            request.size,
            request.aspect_ratio,
            thinking_level,
            truncate_for_log(&request.prompt, 100)
        );

        Ok(PreparedRequest {
            endpoint,
            body,
            is_multipart: false,
            summary,
        })
    }

    fn extract_image_source(&self, response_body: &Value) -> Result<String, AIError> {
        // Nano Banana 2 (Gemini) returns:
        // { "candidates": [{ "content": { "parts": [{ "inlineData": { "data": "base64", "mimeType": "image/png" } }] } }] }
        let candidates = response_body
            .pointer("/candidates")
            .and_then(|v| v.as_array());

        if let Some(candidates) = candidates {
            for candidate in candidates {
                if let Some(parts) = candidate.pointer("/content/parts").and_then(|v| v.as_array()) {
                    for part in parts {
                        if let Some(inline_data) = part.get("inlineData") {
                            let data = inline_data
                                .get("data")
                                .and_then(|v| v.as_str())
                                .filter(|v| !v.trim().is_empty());
                            let mime = inline_data
                                .get("mimeType")
                                .and_then(|v| v.as_str())
                                .unwrap_or("image/png");

                            if let Some(data) = data {
                                return Ok(format!("data:{};base64,{}", mime, data));
                            }
                        }
                    }
                }
            }
        }

        Err(AIError::Provider(format!(
            "Nano Banana 2 response has no image data: {}",
            truncate_for_log(&response_body.to_string(), 500)
        )))
    }
}

inventory::submit! {
    crate::ai::providers::apiyi::models::RegisteredApiyiModel {
        build: || Box::new(NanoBanana2Adapter::new()),
    }
}
