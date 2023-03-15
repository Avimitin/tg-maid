use anyhow::Context;
use std::time::Duration;

use crate::data::DataFetcher;

pub struct HttpClient {
    inner: reqwest::Client,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self {
            inner: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    async fn to_t<T, U>(&self, url: U) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
        U: reqwest::IntoUrl + std::fmt::Display,
    {
        // for debugging usage
        let url_str = url.to_string();

        self.inner
            .get(url)
            .send()
            .await
            .with_context(|| format!("fail to send GET request to url: `{}`", url_str))?
            .json::<T>()
            .await
            .with_context(|| {
                format!(
                    "fail to parse response from url: `{}` to type `{}`",
                    url_str,
                    std::any::type_name::<T>()
                )
            })
    }
}

// TODO: impl DataFetcher for HttpClient {}
