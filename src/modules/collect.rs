pub type Message = (String, String);

#[async_trait::async_trait]
pub trait Collector: Send + Sync + Clone {
    async fn push(&mut self, uid: u64, pair: Message) -> anyhow::Result<u32>;
    async fn finish(&mut self, uid: u64) -> Option<String>;
}
