use anyhow::Context;
use std::ops::Deref;
use std::time::Duration;

pub struct HttpClient(#[cfg(feature = "reqwest")] reqwest::Client);

impl Default for HttpClient {
    fn default() -> Self {
        Self(
            #[cfg(feature = "reqwest")]
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        )
    }
}

impl Deref for HttpClient {
    #[cfg(feature = "reqwest")]
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "reqwest")]
    #[inline]
    pub async fn to_t<T>(&self, url: impl reqwest::IntoUrl + std::fmt::Display) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // for debugging usage
        let url_str = url.to_string();

        self.get(url)
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

    #[inline]
    pub fn inner(&self) -> &reqwest::Client {
        &self.0
    }
}
