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

/// The response from MJX API is different. This type can match those different response.
/// And its associate function can help extract the image link from response.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum MjxApiPossibleReponse {
    Uomg { code: u8, imgurl: String },
    Vvhan { title: String, pic: String },
}

impl MjxApiPossibleReponse {
    /// Extract the image url from response
    pub fn unwrap_url(self) -> String {
        match self {
            Self::Uomg { imgurl, .. } => imgurl,
            Self::Vvhan { pic, .. } => pic,
        }
    }
}

// TODO: impl DataFetcher for HttpClient {}
impl HttpClient {
    const KONACHAN_LINK: &str = "https://konachan.com/post.json?limit=200&tags=%20rating:explicit";

    async fn fetch_nsfw_anime_img(&self) -> anyhow::Result<Sendable> {
        let response: Vec<KonachanApiResponse> = self
            .to_t(Self::KONACHAN_LINK)
            .await
            .with_context(|| "fail to get resp from konachan API")?;

        let mut choice = rand::thread_rng();
        let choice = choice.gen_range(0..response.len());
        let response = &response[choice];

        let sendable = Sendable::builder()
            .url(&response.jpeg_url)
            .caption(format!(
                "<a href=\"{}\">Download Link</a>\nSize: {:.2} MB, Author: {}",
                response.file_url,
                response.file_size as f32 / 1000000.0,
                response.author
            ))
            .build();

        Ok(sendable)
    }

    // TODO: replace the implementation: Get AI generated image from Civitai
    async fn fetch_nsfw_photo(&self) -> anyhow::Result<Sendable> {
        let fallbacks_urls = [
            "https://api.uomg.com/api/rand.img3?format=json",
            "https://api.vvhan.com/api/tao?type=json",
        ];

        let mut trace = Vec::new();

        for url in fallbacks_urls {
            match self.to_t::<MjxApiPossibleReponse, _>(url).await {
                Ok(res) => return Ok(Sendable::builder().url(res.unwrap_url()).build()),

                Err(e) => {
                    trace.push(e.to_string());
                }
            }
        }

        anyhow::bail!(
            "fail to make request to all TaoBao API: {}",
            trace.join("\n\n")
        )
    }
}
