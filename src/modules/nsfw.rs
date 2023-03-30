use crate::app::AppData;

use super::Sendable;
use anyhow::Context;
use rand::Rng;
use serde::Deserialize;

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

const KONACHAN_LINK: &str = "https://konachan.com/post.json?limit=200&tags=%20rating:explicit";

/// Returns a random pick nsfw anime img from konachan
///
/// # Errors
///
/// This function will return an error if http request fail.
pub async fn fetch_nsfw_anime_img(data: AppData) -> anyhow::Result<Sendable> {
    let response: Vec<KonachanApiResponse> = data
        .requester
        .to_t(KONACHAN_LINK)
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

/// Returns a random nsfw image from Taobao Maijiaxiu.
///
/// # Errors
///
/// This function will return an error if http request fail.
// TODO: replace the implementation: Get AI generated image from Civitai
pub async fn fetch_nsfw_photo(data: AppData) -> anyhow::Result<Sendable> {
    let fallbacks_urls = [
        "https://api.uomg.com/api/rand.img3?format=json",
        "https://api.vvhan.com/api/tao?type=json",
    ];

    let mut trace = Vec::new();

    for url in fallbacks_urls {
        match data.requester.to_t::<MjxApiPossibleReponse>(url).await {
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
