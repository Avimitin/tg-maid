use std::collections::HashMap;
pub type Pair = (String, String);

#[async_trait::async_trait]
pub trait Collector: Send + Sync + Clone {
    async fn push(&mut self, uid: u64, pair: Pair) -> anyhow::Result<u32>;
    async fn finish(&mut self, uid: u64) -> Option<String>;
}

#[derive(Clone)]
pub struct InMemMsgCollector {
    storage: std::sync::Arc<tokio::sync::Mutex<HashMap<u64, Vec<Pair>>>>,
}

impl InMemMsgCollector {
    pub fn new() -> Self {
        Self {
            storage: std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl Collector for InMemMsgCollector {
    async fn push(&mut self, uid: u64, pair: Pair) -> anyhow::Result<u32> {
        let mut storage = self.storage.lock().await;
        let entry = storage.entry(uid).or_insert(Vec::new());
        entry.push(pair);
        return Ok(entry.len() as u32);
    }

    async fn finish(&mut self, uid: u64) -> Option<String> {
        let mut storage = self.storage.lock().await;
        let collection = storage.remove(&uid)?;
        Some(collection.iter().fold(String::new(), |res, cur| {
            format!("{res}\n{}: {}", cur.0, cur.1)
        }))
    }
}
