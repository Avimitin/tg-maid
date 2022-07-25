#[async_trait::async_trait]
pub trait KsyxCounter: Send + Sync + Clone {
    async fn add(&mut self) -> anyhow::Result<u32>;
}
