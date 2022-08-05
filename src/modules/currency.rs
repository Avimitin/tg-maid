// ----------------- Types ----------------
use std::collections::HashMap; 
use serde::Deserialize;
use anyhow::Context;

/// The actual rate information during the runtime
#[derive(Debug)]
pub struct CurrencyRateInfo {
    pub date: String,
    pub rate: f64,
}

impl CurrencyRateInfo {
    pub fn new(date: String, rate: f64) -> Self {
        Self { date, rate }
    }
}

/// Possible value return from API
#[derive(Deserialize)]
#[serde(untagged)]
pub enum CurrencyV1PossibleResponse {
    Float(f64),
    String(String),
}

impl CurrencyV1PossibleResponse {
    pub fn unwrap_rate(&self) -> f64 {
        match self {
            Self::Float(f) => *f,
            _ => panic!("currency return non-float rate"),
        }
    }

    pub fn unwrap_date(&self) -> String {
        match self {
            Self::String(s) => s.to_string(),
            _ => panic!("currency return non-string date"),
        }
    }
}

/// An async trait that define the behavior of a cache for currencies.
#[async_trait::async_trait]
pub trait CurrenciesCache: Send + Sync + Clone {
    async fn verify_date(&mut self) -> bool;
    async fn update_currency_codes(&mut self, codes: HashMap<String, String>);
    async fn get_fullname(&mut self, code: &str) -> Option<String>;
}

#[async_trait::async_trait]
pub trait CurrenciesRateProvider {
    type Rate;
    async fn fetch_rate(&self, from: &str, to: &str) -> Self::Rate;
}

#[async_trait::async_trait]
impl CurrenciesRateProvider for crate::butler::Fetcher {
    type Rate = anyhow::Result<CurrencyRateInfo>;

    async fn fetch_rate(&self, from: &str, to: &str) -> Self::Rate {
        // Thanks Asuna (GitHub @SpriteOvO)
        macro_rules! format_array {
            ( [ $( $pattern:literal ),+ $(,)? ] ) => {
                [ $( format!($pattern ) ),+ ]
            };
        }

        let fallbacks_urls = format_array!([
              "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies/{from}/{to}.min.json",
              "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies/{from}/{to}.json",
              "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies/{from}/{to}.min.json",
              "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies/{from}/{to}.json"
        ]);

        let mut error_trace = Vec::new();
        for url in &fallbacks_urls {
            let url = reqwest::Url::parse(url)
                .with_context(|| format!("invalid url input: {from} and {to}"))?;

            match self
                .to_t::<HashMap<String, CurrencyV1PossibleResponse>>(url)
                .await
            {
                Ok(res) => {
                    let rate = res
                        .get(to)
                        .ok_or_else(|| anyhow::anyhow!("fail to get response"))?
                        .unwrap_rate();
                    let date = res
                        .get("date")
                        .expect("Expect response contains date field, but got nil")
                        .unwrap_date();
                    return Ok(CurrencyRateInfo::new(date, rate));
                }
                Err(e) => {
                    error_trace.push(e.to_string());
                }
            }
        }

        anyhow::bail!(
            "fail to send request to all currency API: {}",
            error_trace.join("\n\n")
        )
    }
}
