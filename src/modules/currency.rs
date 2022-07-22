use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct RateInfo {
    date: String,
    rate: f64,
}

#[async_trait::async_trait]
pub trait CurrenciesStorage: Send + Sync + Clone {
    async fn verify_date(&mut self) -> bool;
    async fn update_currency_codes(&mut self, codes: HashMap<String, String>);
    async fn get_fullname(&mut self, code: &str) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct RateCalculator<T: CurrenciesStorage> {
    cache: T,
    api: ApiFetcher,
}

impl<T: CurrenciesStorage> RateCalculator<T> {
    pub fn new(cache: T) -> Self {
        Self {
            cache,
            api: ApiFetcher::new(),
        }
    }

    pub async fn verify_code(&mut self, code: &str) -> bool {
        self.cache.get_fullname(code).await.is_some()
    }

    /// Calculate the currency by rate
    pub async fn calc(&mut self, amount: f64, from: &str, to: &str) -> Result<(f64, String)> {
        if self.cache.verify_date().await {
            let codes = self.api.fetch_latest_code().await?;
            self.cache.update_currency_codes(codes).await;
        }

        if !self.verify_code(from).await {
            anyhow::bail!("invalid code `{from}`")
        }

        if !self.verify_code(to).await {
            anyhow::bail!("invalid code `{to}`")
        }

        let rate_info = self.api.fetch_latest_rate(from, to).await?;
        Ok((rate_info.rate * amount, rate_info.date))
    }

    pub async fn get_fullname(&mut self, code: &str) -> Option<String> {
        self.cache.get_fullname(code).await
    }
}

#[derive(Debug, Clone)]
struct ApiFetcher {
    http_client: reqwest::Client,
}

impl ApiFetcher {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    async fn get(&self, url: &str) -> Result<Vec<u8>> {
        Ok(self
            .http_client
            .get(url)
            .send()
            .await?
            .bytes()
            .await?
            .to_vec())
    }

    pub async fn fetch_latest_code(&self) -> Result<HashMap<String, String>> {
        let fallbacks_urls = [
            "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies.min.json",
            "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies.json",
            "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies.min.json",
            "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies.json"
        ];

        let mut error_trace = Vec::new();
        let mut byte = None;
        for url in fallbacks_urls {
            match self.get(url).await {
                Ok(b) => {
                    byte = Some(b);
                    break;
                }
                Err(e) => {
                    // TODO: Logging
                    error_trace.push(e.to_string())
                }
            }
        }

        if byte.is_none() {
            anyhow::bail!("fail to fetch currencies: {}", error_trace.join("\n\n"))
        }

        let byte = byte.unwrap();
        Ok(serde_json::from_slice(&byte)?)
    }

    pub async fn fetch_latest_rate(&self, from: &str, to: &str) -> Result<RateInfo> {
        macro_rules! format_array {
            ( [ $( $pattern:literal ),+ $(,)? ] ) => {
                [ $( format!($pattern ) ),+ ]
            };
        }

        // Thanks Asuna (GitHub @SpriteOvO)
        let fallbacks_urls = format_array!([
              "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies/{from}/{to}.min.json",
              "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies/{from}/{to}.json",
              "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies/{from}/{to}.min.json",
              "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies/{from}/{to}.json"
        ]);

        let mut error_trace = Vec::new();
        let mut byte = None;
        for url in &fallbacks_urls {
            match self.get(url).await {
                Ok(b) => {
                    byte = Some(b);
                    break;
                }
                Err(e) => {
                    // TODO: logging
                    error_trace.push(e.to_string())
                }
            }
        }

        if byte.is_none() {
            anyhow::bail!(
                "fail to fetch rate for {from}/{to}: {}",
                error_trace.join("\n\n")
            )
        }

        let byte = byte.unwrap();

        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Value {
            Float(f64),
            String(String),
        }

        let response: HashMap<String, Value> = serde_json::from_slice(&byte)?;
        let rate = response
            .get(to)
            .ok_or_else(|| anyhow::anyhow!("fail to get response"))?;
        let date = response
            .get("date")
            .expect("Expect response contains date field, but got nil");

        let rate = match rate {
            Value::Float(f) => f,
            _ => panic!("currency return non-float rate"),
        };

        let date = match date {
            Value::String(s) => s,
            _ => panic!("currency return non-string date"),
        };

        Ok(RateInfo {
            date: date.to_string(),
            rate: *rate,
        })
    }
}
