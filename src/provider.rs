use crate::model::{
    ArchLinuxPkgInfo, ArchLinuxSearchResponse, KonachanApiResponse, MjxApiPossibleReponse,
};
use anyhow::Context;
use rand::Rng;
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

    const ARCH_PKG_SEARCH_API: &str = "https://www.archlinux.org/packages/search/json";

    async fn fetch_pkg_info(&self, pkg: &str, max: usize) -> anyhow::Result<Sendable> {
        let url = reqwest::Url::parse_with_params(Self::ARCH_PKG_SEARCH_API, &[("name", pkg)])
            .with_context(|| format!("{pkg} is a invalid params"))?;

        let resp: ArchLinuxSearchResponse = self.to_t(url).await?;
        if !resp.is_valid() {
            anyhow::bail!("invalid request!")
        }

        let pkg = resp
            .results()
            .iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no result found for {pkg}"))?;

        Ok(Sendable::builder().text(pkg).build())
    }

    async fn fetch_pkg_list(&self, pkg: &str, max: usize) -> anyhow::Result<Sendable> {
        let query_by = |typ: &str| -> anyhow::Result<reqwest::Url> {
            reqwest::Url::parse_with_params(Self::ARCH_PKG_SEARCH_API, &[(typ, pkg)])
                .with_context(|| format!("{pkg} is a invalid params"))
        };

        let (exact_match, fuzzy_match) = tokio::join! {
             self.to_t::<ArchLinuxSearchResponse, _>(query_by("name")?),
             self.to_t::<ArchLinuxSearchResponse, _>(query_by("q")?),
        };

        let (exact_match, fuzzy_match) = (exact_match?, fuzzy_match?);

        if !exact_match.is_valid() || !fuzzy_match.is_valid() {
            anyhow::bail!("invalid request!")
        }

        // Format characters: 19
        //   + Max repo size (Community): 9
        //   + General pkgname size: 10
        //   + General pkgdesc size: 50
        //
        // * Entry counts: max
        const MAYBE_CHAR_SIZE: usize = 19 + 9 + 10 + 50;
        let fuzzy_pkg_size = fuzzy_match.results().len();
        let maybe_entry_size = if max > fuzzy_pkg_size {
            max
        } else {
            fuzzy_pkg_size
        };
        let mut buffer = String::with_capacity(MAYBE_CHAR_SIZE * maybe_entry_size);

        let mut push_into_buffer = |pkg: &ArchLinuxPkgInfo| {
            let display = format!("<b>{}/{}</b>\n    {}", pkg.repo, pkg.pkgname, pkg.pkgdesc);
            buffer.push_str(&display);
            buffer.push('\n');
        };

        if !exact_match.is_empty() {
            push_into_buffer(&exact_match.results()[0])
        }

        fuzzy_match
            .results()
            .iter()
            .take(max)
            .for_each(push_into_buffer);

        Ok(Sendable::builder().text(buffer).build())
    }
}
