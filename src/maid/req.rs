use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref CURRENCY_CODE_URLS: Vec<reqwest::Url> = {
        vec![
            "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies.min.json",
            "https://cdn.jsdelivr.net/gh/fawazahmed0/currency-api@1/latest/currencies.json",
            "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies.min.json",
            "https://raw.githubusercontent.com/fawazahmed0/currency-api/1/latest/currencies.json"
        ]
        .iter()
        .map(|url| reqwest::Url::parse(url).unwrap())
        .collect()
    };
}

/// A wrapper for re-using the reqwest client.
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) c: std::sync::Arc<reqwest::Client>,
}

impl std::default::Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            c: std::sync::Arc::new(
                reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .expect("Fail to build req client"),
            ),
        }
    }

    /// Make a GET request to the given URL and parse the JSON response to given T type.
    /// Please make sure that the given URL will return JSON response.
    ///
    /// # Error
    /// Return error if:
    ///     * Fail to send HTTP request
    ///     * Fail to get response
    ///     * Response is not JSON
    ///     * Fail to parse response into given type
    #[inline]
    pub(crate) async fn to_t<T: serde::de::DeserializeOwned>(
        &self,
        url: reqwest::Url,
    ) -> anyhow::Result<T> {
        Ok(self.c.get(url).send().await?.json::<T>().await?)
    }

    #[inline]
    pub(crate) async fn fetch(&self, url: reqwest::Url) -> anyhow::Result<String> {
        Ok(self.c.get(url).send().await?.text().await?)
    }

    /// Helper function for getting the Currency V1 currencies code.
    pub async fn get_currency_codes(&self) -> anyhow::Result<HashMap<String, String>> {
        let mut error_trace = Vec::new();
        for url in CURRENCY_CODE_URLS.iter() {
            match self.to_t::<HashMap<String, String>>(url.clone()).await {
                Ok(codes) => {
                    return Ok(codes);
                }
                Err(e) => {
                    // TODO: Logging
                    error_trace.push(e.to_string())
                }
            }
        }

        anyhow::bail!("fail to fetch currencies: {}", error_trace.join("\n\n"))
    }
}
