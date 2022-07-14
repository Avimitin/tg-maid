use anyhow::Result;

pub trait CurrencyTempStorage {
    fn is_outdated(&self) -> bool;
    fn store<F: Into<String>, T: Into<String>>(&mut self, from: F, to: T, rate: f64) -> Result<()>;
    fn get(&self, from: &str, to: &str) -> Option<f64>;
    fn update_codes(&mut self, codes: HashMap<String, String>);
    fn has_code(&self, code: &str) -> bool;
}

pub struct Currency<T: CurrencyTempStorage> {
    cache: T,
    api: ApiFetcher,
}

impl<T: CurrencyTempStorage> Currency<T> {
    pub fn new(cache: T) -> Self {
        Self {
            cache,
            api: ApiFetcher::new(),
        }
    }

    pub fn is_valid_code(&self, code: &str) -> bool {
        self.cache.has_code(code)
    }

    /// Calculate the currency by rate
    pub async fn calc(&mut self, amount: f64, from: &str, to: &str) -> Result<f64> {
        // FIXME: Can we remove this hierachy in logic?
        let rate = if self.cache.is_outdated() {
            let codes = self.api.fetch_latest_code().await?;
            self.cache.update_codes(codes);
            let rate = self.api.fetch_latest_rate(from, to).await?;
            self.cache.store(from, to, rate)?;

            rate
        } else {
            if let Some(rate) = self.cache.get(from, to) {
                rate
            } else {
                let rate = self.api.fetch_latest_rate(from, to).await?;
                self.cache.store(from, to, rate)?;

                rate
            }
        };

        Ok(rate * amount)
    }
}

use std::collections::HashMap;

pub struct InMemCache {
    last_update: chrono::DateTime<chrono::Utc>,
    rate: HashMap<String, HashMap<String, f64>>,
    codes: HashMap<String, String>,
}

impl CurrencyTempStorage for InMemCache {
    fn store<F: Into<String>, T: Into<String>>(&mut self, from: F, to: T, rate: f64) -> Result<()> {
        let from = from.into();
        let to = to.into();

        let inner = self.rate.entry(from).or_insert(HashMap::new());
        inner.insert(to, rate);

        Ok(())
    }

    fn get(&self, from: &str, to: &str) -> Option<f64> {
        let inner = self.rate.get(from)?;
        inner.get(to).map(|rate| *rate)
    }

    fn is_outdated(&self) -> bool {
        let now = chrono::Utc::now();

        now - self.last_update > chrono::Duration::days(1)
    }

    fn update_codes(&mut self, codes: HashMap<String, String>) {
        self.codes = codes
    }

    fn has_code(&self, code: &str) -> bool {
        self.codes.get(code).is_some()
    }
}

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

    pub async fn fetch_latest_rate(&self, from: &str, to: &str) -> Result<f64> {
        macro_rules! format_array {
            ( [ $( $pattern:literal ),+ $(,)? ] ) => {
                [ $( format!($pattern ) ),+ ]
            };
        }

        // Thanks Asuna (GitHub @SpiriteOvO)
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

        let response: HashMap<String, String> = serde_json::from_slice(&byte)?;
        Ok(response
            .get(to)
            .ok_or_else(|| anyhow::anyhow!("fail to get response"))?
            .parse()?)
    }
}
