use anyhow::Context;
use rand::Rng;

/// A wrapper for re-using the reqwest client.
#[derive(Debug)]
pub struct Client {
    c: reqwest::Client,
}

impl std::default::Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            c: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Fail to build req client"),
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
    async fn to_t<T: serde::de::DeserializeOwned>(&self, url: reqwest::Url) -> anyhow::Result<T> {
        Ok(self.c.get(url).send().await?.json::<T>().await?)
    }

    pub async fn konachan_explicit_nsfw_image(&self) -> anyhow::Result<(reqwest::Url, String)> {
        const LINK: &str = "https://konachan.com/post.json?limit=200&tags=%20rating:explicit";
        let link = reqwest::Url::parse(LINK).unwrap();

        use crate::modules::types::KonachanApiResponse;
        let response = self
            .to_t::<Vec<KonachanApiResponse>>(link)
            .await
            .with_context(|| "fail to get resp from konachan API")?;

        let mut choice = rand::thread_rng();
        let choice = choice.gen_range(0..response.len());
        let response = &response[choice];

        Ok((
            reqwest::Url::parse(&response.jpeg_url)?,
            format!(
                "<a href=\"{}\">Download Link</a>\nSize: {:.2} MB, Author: {}",
                response.file_url,
                response.file_size as f32 / 1000000.0,
                response.author
            ),
        ))
    }
}
