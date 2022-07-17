use crate::currency::CurrenciesStorage;
use redis::{aio::Connection, AsyncCommands};
use std::collections::HashMap;

#[async_trait::async_trait]
impl CurrenciesStorage for Connection {
    async fn update(&mut self, codes: HashMap<String, String>) {
        for (k, v) in codes {
            match self.hset("currency-code", k, v).await {
                Err(_e) => {}
                Ok(()) => {}
            }
        }

        match self
            .set("currency-last-update", chrono::Utc::now().to_string())
            .await
        {
            Err(_e) => {}
            Ok(()) => {}
        };
    }

    async fn is_outdated(&mut self) -> bool {
        let last_date: Option<String> = self.get("currency-last-update").await.ok();
        if let Some(last_date) = last_date {
            todo!()
        }
        false
    }

    async fn get_fullname(&mut self, code: &str) -> Option<String> {
        self.hget("currency-code", code).await.ok()
    }
}
