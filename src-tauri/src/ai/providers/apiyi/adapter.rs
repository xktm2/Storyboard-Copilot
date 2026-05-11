use serde_json::Value;

use crate::ai::error::AIError;
use crate::ai::GenerateRequest;

pub struct PreparedRequest {
    pub endpoint: String,
    pub body: Value,
    pub is_multipart: bool,
    pub summary: String,
}

pub trait ApiyiModelAdapter: Send + Sync {
    fn model_aliases(&self) -> &'static [&'static str];

    fn canonical_model(&self) -> &'static str {
        self.model_aliases()
            .iter()
            .find(|model| model.contains('/'))
            .copied()
            .or_else(|| self.model_aliases().first().copied())
            .unwrap_or("unknown")
    }

    fn matches(&self, model: &str) -> bool {
        self.model_aliases().iter().any(|alias| alias == &model)
    }

    fn build_request(
        &self,
        request: &GenerateRequest,
        base_url: &str,
    ) -> Result<PreparedRequest, AIError>;

    fn extract_image_source(&self, response_body: &Value) -> Result<String, AIError>;
}
