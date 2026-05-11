use super::adapter::ApiyiModelAdapter;

automod::dir!("src/ai/providers/apiyi/models");

pub struct RegisteredApiyiModel {
    pub build: fn() -> Box<dyn ApiyiModelAdapter>,
}

inventory::collect!(RegisteredApiyiModel);

pub fn collect_adapters() -> Vec<Box<dyn ApiyiModelAdapter>> {
    inventory::iter::<RegisteredApiyiModel>
        .into_iter()
        .map(|entry| (entry.build)())
        .collect()
}
