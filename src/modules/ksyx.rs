#[async_trait::async_trait]
pub trait KsyxCounterCache: Send + Sync + Clone {
    async fn hit(&mut self) -> anyhow::Result<u32>;
}
