use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::ai::error::AIError;
use crate::ai::GenerateRequest;

use super::super::adapter::{ApiyiModelAdapter, PreparedRequest};

const API_MODEL_NAME: &str = "gemini-3-pro-image-preview";

pub struct NanoBananaProAdapter;

impl NanoBananaProAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NanoBananaProAdapter {
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

    // Already a data URL — extract the base64 payload
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

fn truncate_for_log(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>()
}

impl ApiyiModelAdapter for NanoBananaProAdapter {
    fn model_aliases(&self) -> &'static [&'static str] {
        &["apiyi/nano-banana-pro", "nano-banana-pro"]
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

        // NB Pro: always use inlineData (base64), no thinkingConfig
        let mut parts = vec![json!({ "text": request.prompt })];

        if has_reference_images {
            let reference_images = request.reference_images.as_deref().unwrap_or(&[]);
            for source in reference_images {
                let base64_data = source_to_base64(source).map_err(|err| {
                    AIError::InvalidRequest(format!(
                        "Failed to encode reference image for Nano Banana Pro: {}",
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

        let body = json!({
            "contents": [{
                "parts": parts
            }],
            "generationConfig": {
                "responseModalities": ["IMAGE"],
                "imageConfig": {
                    "imageSize": request.size,
                    "aspectRatio": request.aspect_ratio
                }
            }
        });

        let endpoint = format!(
            "{}/v1beta/models/{}:generateContent",
            base_url, API_MODEL_NAME
        );

        let mode_label = if has_reference_images { "edit" } else { "generate" };
        let summary = format!(
            "model: apiyi/nano-banana-pro, mode: {}, size: {}, aspect_ratio: {}, prompt: {}",
            mode_label,
            request.size,
            request.aspect_ratio,
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
        // Check for content moderation rejection
        if let Some(candidates) = response_body.pointer("/candidates").and_then(|v| v.as_array()) {
            for candidate in candidates {
                // finishReason != STOP indicates generation-time rejection
                if let Some(reason) = candidate.get("finishReason").and_then(|v| v.as_str()) {
                    if reason != "STOP" {
                        return Err(AIError::Provider(format!(
                            "Nano Banana Pro generation rejected: finishReason={}", reason
                        )));
                    }
                }
            }
        }

        // Check candidatesTokenCount == 0 (review-stage rejection)
        if let Some(token_count) = response_body
            .pointer("/usageMetadata/candidatesTokenCount")
            .and_then(|v| v.as_i64())
        {
            if token_count == 0 {
                return Err(AIError::Provider(
                    "Nano Banana Pro content rejected during review (candidatesTokenCount=0)".to_string(),
                ));
            }
        }

        // Same response format as NB2
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
            "Nano Banana Pro response has no image data: {}",
            truncate_for_log(&response_body.to_string(), 500)
        )))
    }
}

inventory::submit! {
    crate::ai::providers::apiyi::models::RegisteredApiyiModel {
        build: || Box::new(NanoBananaProAdapter::new()),
    }
}
