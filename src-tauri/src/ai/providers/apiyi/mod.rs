pub mod adapter;
pub mod models;
pub mod registry;

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::ai::error::AIError;
use crate::ai::AIProvider;
use crate::ai::{GenerateRequest, ProviderTaskHandle, ProviderTaskPollResult, ProviderTaskSubmission};

use registry::ApiyiModelRegistry;

const DEFAULT_BASE_URL: &str = "https://api.apiyi.com";

pub struct ApiyiProvider {
    client: Client,
    api_key: Arc<RwLock<Option<String>>>,
    base_url: String,
    model_registry: ApiyiModelRegistry,
}

impl ApiyiProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: Arc::new(RwLock::new(None)),
            base_url: DEFAULT_BASE_URL.to_string(),
            model_registry: ApiyiModelRegistry::new(),
        }
    }

    async fn get_api_key(&self) -> Result<String, AIError> {
        self.api_key
            .read()
            .await
            .clone()
            .ok_or_else(|| AIError::InvalidRequest("API key not set".to_string()))
    }
}

impl Default for ApiyiProvider {
    fn default() -> Self {
        Self::new()
    }
}

// --- Shared helpers for multipart image sources ---

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

/// Build a FLUX.2 Max multipart form for edits endpoint
fn build_flux_multipart_form(
    request: &GenerateRequest,
    size: &str,
) -> Result<Form, AIError> {
    let reference_images = request.reference_images.as_deref().unwrap_or(&[]);
    let mut form = Form::new()
        .text("model", "flux-2-max".to_string())
        .text("prompt", request.prompt.clone())
        .text("n", "1".to_string())
        .text("size", size.to_string())
        .text("output_format", "png".to_string());

    // First image: file upload via "image" field
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

    // Subsequent images: URL strings in input_image_2, input_image_3, ...
    for (i, source) in reference_images.iter().skip(1).enumerate() {
        let field_name = format!("input_image_{}", i + 2);
        if is_http_url(source) {
            form = form.text(field_name, source.clone());
        } else {
            return Err(AIError::InvalidRequest(
                format!("FLUX.2 Max subsequent reference images (input_image_{}+) must be URLs", i + 2)
            ));
        }
    }

    Ok(form)
}

/// Build a GPT Image 2 multipart form for edits endpoint
fn build_gpt_multipart_form(
    request: &GenerateRequest,
    size: &str,
) -> Result<Form, AIError> {
    let reference_images = request.reference_images.as_deref().unwrap_or(&[]);
    let mut form = Form::new()
        .text("model", "gpt-image-2".to_string())
        .text("prompt", request.prompt.clone())
        .text("size", size.to_string())
        .text("quality", "auto".to_string());

    // All images as image[] fields
    for source in reference_images {
        let bytes = source_to_bytes(source).map_err(|err| {
            AIError::InvalidRequest(format!(
                "Failed to read reference image for GPT Image 2: {}",
                err
            ))
        })?;
        let part = Part::bytes(bytes)
            .file_name("image.png")
            .mime_str("image/png")
            .map_err(|e| AIError::InvalidRequest(format!("mime error: {}", e)))?;
        form = form.part("image[]", part);
    }

    Ok(form)
}

/// Download a temporary URL and convert to data URL
async fn download_url_to_data_url(client: &Client, url: &str) -> Result<String, AIError> {
    info!("[APIYI] Downloading temporary URL: {}...", &url[..url.len().min(80)]);
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(AIError::Provider(format!(
            "Failed to download image from URL: {}",
            response.status()
        )));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();

    let bytes = response.bytes().await?;
    let b64 = STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", content_type, b64))
}

#[async_trait::async_trait]
impl AIProvider for ApiyiProvider {
    fn name(&self) -> &str {
        "apiyi"
    }

    fn supports_model(&self, model: &str) -> bool {
        self.model_registry.supports(model)
    }

    fn list_models(&self) -> Vec<String> {
        self.model_registry.list_models()
    }

    async fn set_api_key(&self, api_key: String) -> Result<(), AIError> {
        let mut key = self.api_key.write().await;
        *key = Some(api_key);
        Ok(())
    }

    async fn submit_task(
        &self,
        request: GenerateRequest,
    ) -> Result<ProviderTaskSubmission, AIError> {
        // APIYI uses synchronous requests, so submit just runs generate
        let result = self.generate(request).await?;
        Ok(ProviderTaskSubmission::Succeeded(result))
    }

    async fn poll_task(
        &self,
        _handle: ProviderTaskHandle,
    ) -> Result<ProviderTaskPollResult, AIError> {
        // APIYI is synchronous, no polling needed
        Err(AIError::Provider(
            "APIYI provider does not support task polling".to_string(),
        ))
    }

    async fn generate(&self, request: GenerateRequest) -> Result<String, AIError> {
        let api_key = self.get_api_key().await?;
        let adapter = self
            .model_registry
            .resolve(&request.model)
            .ok_or_else(|| AIError::ModelNotSupported(request.model.clone()))?;

        let prepared = adapter.build_request(&request, &self.base_url)?;
        info!("[APIYI Request] {}", prepared.summary);

        if prepared.is_multipart {
            // Determine which multipart builder to use based on model
            let model_bare = request
                .model
                .split_once('/')
                .map(|(_, m)| m)
                .unwrap_or(&request.model);

            let form = match model_bare {
                "flux-2-max" => {
                    let size = resolve_flux_size(&request.size, &request.aspect_ratio);
                    build_flux_multipart_form(&request, &size)?
                }
                "gpt-image-2" => {
                    let size = resolve_gpt_size(&request.size, &request.aspect_ratio);
                    build_gpt_multipart_form(&request, &size)?
                }
                _ => {
                    return Err(AIError::InvalidRequest(format!(
                        "No multipart builder for model: {}",
                        request.model
                    )));
                }
            };

            let response = self
                .client
                .post(&prepared.endpoint)
                .header("Authorization", format!("Bearer {}", api_key))
                .multipart(form)
                .timeout(std::time::Duration::from_secs(360))
                .send()
                .await
                .map_err(|err| {
                    AIError::Provider(format!(
                        "APIYI multipart request to {} failed: {} (is_timeout={}, is_connect={}, is_body={})",
                        prepared.endpoint,
                        err,
                        err.is_timeout(),
                        err.is_connect(),
                        err.is_body()
                    ))
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(AIError::Provider(format!(
                    "APIYI multipart request failed {}: {}",
                    status, error_text
                )));
            }

            let response_body: Value = response.json().await?;
            let image_source = adapter.extract_image_source(&response_body)?;
            return normalize_image_source(&self.client, &image_source).await;
        }

        // JSON request (text-to-image or Nano Banana 2)
        let response = self
            .client
            .post(&prepared.endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&prepared.body)
            .timeout(std::time::Duration::from_secs(360))
            .send()
            .await
            .map_err(|err| {
                AIError::Provider(format!(
                    "APIYI request to {} failed: {} (is_timeout={}, is_connect={}, is_body={})",
                    prepared.endpoint,
                    err,
                    err.is_timeout(),
                    err.is_connect(),
                    err.is_body()
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AIError::Provider(format!(
                "APIYI request failed {}: {}",
                status, error_text
            )));
        }

        let response_body: Value = response.json().await?;
        let image_source = adapter.extract_image_source(&response_body)?;
        normalize_image_source(&self.client, &image_source).await
    }
}

/// If the image source is a temporary URL, download it and return a data URL.
/// If it's already a data URL, return as-is.
async fn normalize_image_source(client: &Client, source: &str) -> Result<String, AIError> {
    if source.starts_with("data:") {
        return Ok(source.to_string());
    }

    if source.starts_with("http://") || source.starts_with("https://") {
        return download_url_to_data_url(client, source).await;
    }

    Ok(source.to_string())
}

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
