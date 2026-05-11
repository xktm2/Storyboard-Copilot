use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::multipart::{Form, Part};
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::ai::error::AIError;
use crate::ai::GenerateRequest;

use super::super::adapter::{ApiyiModelAdapter, PreparedRequest};

pub struct Flux2MaxAdapter;

impl Flux2MaxAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Flux2MaxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolution + aspect ratio → pixel size string for FLUX.2 Max
fn resolve_flux_size(resolution: &str, aspect_ratio: &str) -> String {
    let sizes: &[(&str, &[(&str, &str)])] = &[
        ("2MP", &[
            ("1:1", "1440x1440"),
            ("4:3", "1600x1200"),
            ("3:4", "1200x1600"),
            ("16:9", "1920x1080"),
            ("9:16", "1080x1920"),
            ("3:2", "1536x1024"),
            ("2:3", "1024x1536"),
            ("21:9", "2240x960"),
        ]),
        ("4MP", &[
            ("1:1", "2048x2048"),
            ("4:3", "2304x1728"),
            ("3:4", "1728x2304"),
            ("16:9", "2560x1440"),
            ("9:16", "1440x2560"),
            ("3:2", "2304x1536"),
            ("2:3", "1536x2304"),
            ("21:9", "2912x1248"),
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

fn source_to_bytes(source: &str) -> Result<Vec<u8>, String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err("source is empty".to_string());
    }

    if let Some((meta, payload)) = trimmed.split_once(',') {
        if meta.starts_with("data:") && meta.ends_with(";base64") && !payload.is_empty() {
            return STANDARD
                .decode(payload)
                .map_err(|err| format!("invalid data-url base64 payload: {}", err));
        }
    }

    let likely_base64 = trimmed.len() > 256
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '=');
    if likely_base64 {
        return STANDARD
            .decode(trimmed)
            .map_err(|err| format!("invalid base64 payload: {}", err));
    }

    let path = if trimmed.starts_with("file://") {
        PathBuf::from(decode_file_url_path(trimmed))
    } else {
        PathBuf::from(trimmed)
    };
    std::fs::read(&path).map_err(|err| {
        format!(
            "failed to read path \"{}\": {}",
            path.to_string_lossy(),
            err
        )
    })
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

impl ApiyiModelAdapter for Flux2MaxAdapter {
    fn model_aliases(&self) -> &'static [&'static str] {
        &["apiyi/flux-2-max", "flux-2-max"]
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

        let size = resolve_flux_size(&request.size, &request.aspect_ratio);

        if has_reference_images {
            // FLUX.2 Max edits: multipart/form-data
            // image field = first ref as file upload
            // input_image_2..8 = subsequent refs as URL strings
            let reference_images = request.reference_images.as_deref().unwrap_or(&[]);
            let mut form = Form::new()
                .text("model", "flux-2-max".to_string())
                .text("prompt", request.prompt.clone())
                .text("n", "1".to_string())
                .text("size", size.clone())
                .text("output_format", "png".to_string());

            // First image: file upload
            if let Some(first_source) = reference_images.first() {
                if is_http_url(first_source) {
                    form = form.text("image", first_source.clone());
                } else {
                    let bytes = source_to_bytes(first_source).map_err(|err| {
                        AIError::InvalidRequest(format!(
                            "Failed to read first reference image for FLUX: {}",
                            err
                        ))
                    })?;
                    let part = Part::bytes(bytes)
                        .file_name("image.png")
                        .mime_str("image/png")
                        .map_err(|e| AIError::InvalidRequest(format!("mime error: {}", e)))?;
                    form = form.part("image", part);
                }
            }

            // Subsequent images: URL strings in form fields
            for (i, source) in reference_images.iter().skip(1).enumerate() {
                let field_name = format!("input_image_{}", i + 2);
                let url = if is_http_url(source) {
                    source.clone()
                } else {
                    // Non-URL sources need to be uploaded somehow;
                    // for FLUX edits, subsequent images must be URLs
                    return Err(AIError::InvalidRequest(
                        format!("FLUX.2 Max subsequent reference images (input_image_{}+) must be URLs, got non-URL source", i + 2)
                    ));
                };
                form = form.text(field_name, url);
            }

            let summary = format!(
                "model: apiyi/flux-2-max, mode: edit, images: {}, size: {}, prompt: {}",
                reference_images.len(),
                size,
                truncate_for_log(&request.prompt, 100)
            );

            // For multipart, we store the form in metadata and handle it in the provider
            Ok(PreparedRequest {
                endpoint: format!("{}/v1/images/edits", base_url),
                body: json!({ "__multipart__": true }),
                is_multipart: true,
                summary,
            })
        } else {
            // Text-to-image: JSON body
            let body = json!({
                "model": "flux-2-max",
                "prompt": request.prompt,
                "n": 1,
                "size": size,
                "output_format": "png"
            });

            let summary = format!(
                "model: apiyi/flux-2-max, mode: generate, size: {}, prompt: {}",
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
        // FLUX returns: { "data": [{ "url": "https://..." }] }
        let url = response_body
            .pointer("/data/0/url")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty());

        if let Some(url) = url {
            return Ok(url.to_string());
        }

        Err(AIError::Provider(format!(
            "FLUX.2 Max response has no image URL: {}",
            response_body
        )))
    }
}

fn truncate_for_log(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>()
}

inventory::submit! {
    crate::ai::providers::apiyi::models::RegisteredApiyiModel {
        build: || Box::new(Flux2MaxAdapter::new()),
    }
}
