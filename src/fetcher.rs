use anyhow::Context;
use rand::Rng;
use serde::Deserialize;
use std::time::Duration;

use crate::data::{DataFetcher, Sendable};

pub struct HttpClient {
    #[cfg(feature = "reqwest")]
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

/// Represent the konachan API response json
#[derive(Deserialize, Debug)]
pub struct KonachanApiResponse {
    pub jpeg_url: String,
    pub file_url: String,
    pub file_size: u32,
    pub author: String,
}

// TODO: impl DataFetcher for HttpClient {}
impl HttpClient {
    const KONACHAN_LINK: &str = "https://konachan.com/post.json?limit=200&tags=%20rating:explicit";

    async fn fetch_nsfw_anime_img(&self) -> anyhow::Result<Sendable> {
        let link = reqwest::Url::parse(Self::KONACHAN_LINK).unwrap();

        let response: Vec<KonachanApiResponse> = self
            .to_t(link)
            .await
            .with_context(|| "fail to get resp from konachan API")?;

        let mut choice = rand::thread_rng();
        let choice = choice.gen_range(0..response.len());
        let response = &response[choice];

        Ok(Sendable::from_url_and_caption(
            &response.jpeg_url,
            format!(
                "<a href=\"{}\">Download Link</a>\nSize: {:.2} MB, Author: {}",
                response.file_url,
                response.file_size as f32 / 1000000.0,
                response.author
            ),
        ))
    }
}
