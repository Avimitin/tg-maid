use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represent the konachan API response json
#[derive(Deserialize, Debug)]
pub struct KonachanApiResponse {
    pub jpeg_url: String,
    pub file_url: String,
    pub file_size: u32,
    pub author: String,
}

#[derive(Debug)]
pub struct CurrencyRateInfo {
    pub date: String,
    pub rate: f64,
}

impl CurrencyRateInfo {
    pub fn new(date: String, rate: f64) -> Self {
        Self { date, rate }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum CurrencyV1PossibleResponse {
    Float(f64),
    String(String),
}

impl CurrencyV1PossibleResponse {
    pub fn unwrap_rate(&self) -> f64 {
        match self {
            Self::Float(f) => *f,
            _ => panic!("currency return non-float rate"),
        }
    }

    pub fn unwrap_date(&self) -> String {
        match self {
            Self::String(s) => s.to_string(),
            _ => panic!("currency return non-string date"),
        }
    }
}

#[async_trait::async_trait]
pub trait CurrenciesStorage: Send + Sync + Clone {
    async fn verify_date(&mut self) -> bool;
    async fn update_currency_codes(&mut self, codes: HashMap<String, String>);
    async fn get_fullname(&mut self, code: &str) -> Option<String>;
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

/// Types for sending request to ehentai
#[derive(Serialize, Debug)]
pub struct EhentaiRequestType<'a> {
    method: String,
    namespace: u8,
    gidlist: &'a [[String; 2]],
}

impl<'a> EhentaiRequestType<'a> {
    /// Require gid_list to consturct the request.
    /// Read https://ehwiki.org/wiki/API for details.
    pub fn new(gidlist: &'a [[String; 2]]) -> Self {
        Self {
            gidlist,
            method: "gdata".to_string(),
            namespace: 1,
        }
    }
}

fn to_u32<'de, D>(d: D) -> Result<u32, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let orig: String = serde::de::Deserialize::deserialize(d)?;
    use serde::de::Error;
    orig.parse::<u32>().map_err(D::Error::custom)
}

fn to_u64<'de, D>(d: D) -> Result<u64, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let orig: String = serde::de::Deserialize::deserialize(d)?;
    use serde::de::Error;
    orig.parse::<u64>().map_err(D::Error::custom)
}

fn to_f32<'de, D>(d: D) -> Result<f32, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let orig: String = serde::de::Deserialize::deserialize(d)?;
    use serde::de::Error;
    orig.parse::<f32>().map_err(D::Error::custom)
}

fn to_url<'de, D>(d: D) -> Result<reqwest::Url, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let orig: String = serde::de::Deserialize::deserialize(d)?;
    use serde::de::Error;
    reqwest::Url::parse(&orig).map_err(D::Error::custom)
}

/// Represent data for single comic query
#[derive(Deserialize, Debug)]
pub struct EhGmetadata {
    gid: u32,
    title_jpn: String,
    category: String,
    #[serde(deserialize_with = "to_url")]
    pub thumb: reqwest::Url,
    #[serde(deserialize_with = "to_u32")]
    filecount: u32,
    #[serde(deserialize_with = "to_f32")]
    rating: f32,
    tags: Vec<String>,
    #[serde(deserialize_with = "to_u32")]
    torrentcount: u32,
    torrents: Vec<EhTorrent>,
    first_gid: Option<String>,
}

impl EhGmetadata {
    pub fn torrent_to_string(&self, max: usize) -> String {
        let gid = if let Some(ref id) = self.first_gid {
            id.to_string()
        } else {
            format!("{}", self.gid)
        };

        self.torrents
            .iter()
            .take(max)
            .fold(String::new(), |sum, torrent| {
                format!(
                    r#"{sum}
* Name: {}
    Size: {} MB
    Link: https://ehtracker.org/get/{}/{}.torrent
"#,
                    torrent.name,
                    torrent.fsize / 1000000,
                    gid,
                    torrent.hash
                )
            })
    }
}

impl std::fmt::Display for EhGmetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"📖 Title: {}
🗂️ Category: {}
📄 Pages: {}
⭐ Rating: {}
🌱 Torrent Amount: {}
🔖 Tags: {}
"#,
            self.title_jpn,
            self.category,
            self.filecount,
            self.rating,
            self.torrentcount,
            self.tags.iter().fold(String::new(), |acc, x| format!(
                "{acc} #{}",
                x.split(':').nth(1).unwrap().replace(' ', "_").replace('-', "_")
            ))
        )
    }
}

/// Represent the torrent data for one comic
#[derive(Deserialize, Debug)]
struct EhTorrent {
    hash: String,
    name: String,
    #[serde(deserialize_with = "to_u64")]
    fsize: u64,
}

/// The main response
#[derive(Deserialize, Debug)]
pub struct EhentaiMetadataResponse {
    pub gmetadata: Vec<EhGmetadata>,
}

#[derive(Deserialize, Debug)]
pub struct EhentaiReponseError {
    error: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum PossibleEhentaiResponse {
    Norm(EhentaiMetadataResponse),
    Err(EhentaiReponseError),
}

impl PossibleEhentaiResponse {
    pub fn try_unwrap(self) -> anyhow::Result<EhentaiMetadataResponse> {
        match self {
            Self::Norm(res) => Ok(res),
            Self::Err(e) => Err(anyhow::anyhow!("{}", e.error)),
        }
    }
}
