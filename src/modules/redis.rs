use crate::modules::collect::{CollectedMsgCache, MsgForm};
use crate::modules::ksyx::KsyxCounterCache;
use crate::modules::types::CurrenciesCache;
use redis::{aio::ConnectionManager, AsyncCommands};
use std::collections::HashMap;
use tracing::error;

const DATE_FORMAT: &str = "%Y-%m-%d-%H-%M-%S";
const CODE_PREFIX_KEY: &str = "currency-code";
const DATE_KEY: &str = "currency-last-update";

// FIXME: reimplement trait for redis client
#[async_trait::async_trait]
impl CurrenciesCache for ConnectionManager {
    async fn update_currency_codes(&mut self, codes: HashMap<String, String>) {
        for (k, v) in codes {
            let response: Result<(), _> = self.hset(CODE_PREFIX_KEY, k, v).await;
            if let Err(e) = response {
                error!("fail to make hset request to redis: {e}")
            }
        }

        let date = chrono::Utc::now().format(DATE_FORMAT).to_string();
        let response: Result<(), _> = self.set("currency-last-update", date).await;
        if let Err(e) = response {
            error!("fail to make hset request to redis: {e}")
        }
    }

    async fn verify_date(&mut self) -> bool {
        let last_date: Option<String> = self.get(DATE_KEY).await.ok();
        if let Some(last_date) = last_date {
            let date = chrono::NaiveDateTime::parse_from_str(&last_date, DATE_FORMAT)
                .expect("Fail to parse date from redis cache");
            let now = chrono::Utc::now().naive_utc();
            return now - date > chrono::Duration::days(1);
        }
        false
    }

    async fn get_fullname(&mut self, code: &str) -> Option<String> {
        self.hget("currency-code", code).await.ok()
    }
}

#[async_trait::async_trait]
impl CollectedMsgCache for ConnectionManager {
    async fn push(&mut self, uid: u64, pair: MsgForm) -> anyhow::Result<u32> {
        let size: u32 = self
            .rpush(uid, format!("{}: {}", pair.sender, pair.text))
            .await?;
        Ok(size)
    }

    async fn finish(&mut self, uid: u64) -> Option<String> {
        let result: Vec<String> = self.lrange(uid, 0, -1).await.ok()?;
        self.del(uid).await.ok()?;
        Some(
            result
                .iter()
                .fold(String::new(), |acc, x| format!("{}\n{}", acc, x)),
        )
    }
}

#[async_trait::async_trait]
impl KsyxCounterCache for ConnectionManager {
    async fn hit(&mut self) -> anyhow::Result<u32> {
        Ok(self.incr("KSYX_HIT_COUNTER", 1).await?)
    }
}
