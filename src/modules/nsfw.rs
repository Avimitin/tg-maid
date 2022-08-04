use async_trait::async_trait;
use anyhow::Context;
use rand::Rng;
use serde::Deserialize;
use crate::butler::Fetcher;

/// NsfwContentFetcher require two type of output. One should return
/// Anime Waifu uwu, another one should return porn photograph.
#[async_trait]
pub trait NsfwContentFetcher {
    type AnimeOutput;
    type PhotographOutput;

    async fn get_anime_image(&self) -> Self::AnimeOutput;
    async fn get_photograph(&self) -> Self::PhotographOutput;
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

/// Default impl the NsfwContentFetcher for reqwest::Client
#[async_trait::async_trait]
impl NsfwContentFetcher for Fetcher {
    type AnimeOutput = anyhow::Result<(reqwest::Url, String)>;
    type PhotographOutput = anyhow::Result<reqwest::Url>;

    /// The default implementation use Konachan as the R18 Anime image source
    async fn get_anime_image(&self) -> anyhow::Result<(reqwest::Url, String)> {
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

    /// The default implementation fetch TaoBao image comment from bra/sex toy shop.
    /// This is not a perfic choice for porn photograph, I will try to find another source.
    async fn get_photograph(&self) -> anyhow::Result<reqwest::Url> {
        let fallbacks_urls = [
            "https://api.uomg.com/api/rand.img3?format=json",
            "https://api.vvhan.com/api/tao?type=json",
        ];

        let mut trace = Vec::new();

        for url in fallbacks_urls {
            let url = reqwest::Url::parse(url).unwrap();

            match self.to_t::<MjxApiPossibleReponse>(url).await {
                Ok(res) => return Ok(reqwest::Url::parse(&res.unwrap_url())?),

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

