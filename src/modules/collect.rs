pub struct MsgForm {
    pub sender: String,
    pub text: String,
}

#[async_trait::async_trait]
pub trait Collector: Send + Sync + Clone {
    async fn push(&mut self, uid: u64, pair: MsgForm) -> anyhow::Result<u32>;
    async fn finish(&mut self, uid: u64) -> Option<String>;
}
