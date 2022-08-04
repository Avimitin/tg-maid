use serde::{Deserialize, Serialize};

// -------------------------------- Type --------------------------------

/// Data for sending request to ehentai. The `method` field and `namespace`
/// will be genrated automatically.
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

/// Some of the API wrap all the type into string type.
/// This macro rule generate some function that can parse those String
/// response to their actual type.
macro_rules! gen_str_to_t {
    ($($ty:ty),+) => {
        paste::paste! {
            $(
                fn [<to_$ty>]<'de, D>(d: D) -> Result<$ty, D::Error>
                where
                    D: serde::de::Deserializer<'de>,
                {
                    let orig: String = serde::de::Deserialize::deserialize(d)?;
                    use serde::de::Error;
                    orig.parse::<$ty>().map_err(D::Error::custom)
                }
            )+
        }
    };
}

gen_str_to_t!(u32, u64, f32);

/// Parse string to actual URL type
fn to_url<'de, D>(d: D) -> Result<reqwest::Url, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let orig: String = serde::de::Deserialize::deserialize(d)?;
    use serde::de::Error;
    reqwest::Url::parse(&orig).map_err(D::Error::custom)
}

/// Represent data for single comic information
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
    /// Parse the torrents list to Telegram message.
    /// This function will render at most `max` torrent.
    /// If the `max` argument is larger then torrents amount,
    /// all the torrents information will be rendered.
    pub fn to_telegram_html(&self, max: usize) -> String {
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
* <b>Name</b>: <a href="https://ehtracker.org/get/{}/{}.torrent">{}</a>
  <b>Size</b>: {} MB
"#,
                    gid,
                    torrent.hash,
                    torrent.name,
                    torrent.fsize / 1000000,
                )
            })
    }
}

/// Custom format to display comit data.
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
                x.split(':')
                    .nth(1)
                    .unwrap()
                    .replace(' ', "_")
                    .replace('-', "_")
            ))
        )
    }
}

/// Represent a single torrent data
#[derive(Deserialize, Debug)]
struct EhTorrent {
    hash: String,
    name: String,
    #[serde(deserialize_with = "to_u64")]
    fsize: u64,
}

/// Response from ehentai when query succesfully. It contains a list of information
/// for the given gid list.
#[derive(Deserialize, Debug)]
pub struct EhentaiMetadataResponse {
    pub gmetadata: Vec<EhGmetadata>,
}

/// Response from ehentai when query fail.
#[derive(Deserialize, Debug)]
pub struct EhentaiReponseError {
    error: String,
}

/// The main response from ehentai API. Use the `try_unwrap`
/// function to convert the response to a Result type.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum PossibleEhentaiResponse {
    Norm(EhentaiMetadataResponse),
    Err(EhentaiReponseError),
}

impl PossibleEhentaiResponse {
    /// Convert the `PossibleEhentaiResponse` to `anyhow::Result` type.
    pub fn try_unwrap(self) -> anyhow::Result<EhentaiMetadataResponse> {
        match self {
            Self::Norm(res) => Ok(res),
            Self::Err(e) => Err(anyhow::anyhow!("{}", e.error)),
        }
    }
}

#[async_trait::async_trait]
pub trait EhentaiFetcher {
    type Result;
    async fn get_ehentai_comic_data(&self, gid_list: &[[String; 2]]) -> Self::Result;
}

#[async_trait::async_trait]
impl EhentaiFetcher for crate::butler::Fetcher {
    type Result = anyhow::Result<EhentaiMetadataResponse>;

    async fn get_ehentai_comic_data(&self, gid_list: &[[String; 2]]) -> Self::Result {
        let api_url: reqwest::Url =
            reqwest::Url::parse("https://api.e-hentai.org/api.php").unwrap();

        let request_data = EhentaiRequestType::new(gid_list);

        self.c
            .post(api_url)
            .json(&request_data)
            .send()
            .await?
            .json::<PossibleEhentaiResponse>()
            .await?
            .try_unwrap()
    }
}
