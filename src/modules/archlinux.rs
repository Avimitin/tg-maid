use crate::app::AppData;

use super::Sendable;
use anyhow::Context;
use serde::Deserialize;

const ARCH_PKG_SEARCH_API: &str = "https://www.archlinux.org/packages/search/json";

#[derive(Deserialize)]
pub struct ArchLinuxSearchResponse {
    valid: bool,
    results: Vec<ArchLinuxPkgInfo>,
}

impl ArchLinuxSearchResponse {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    #[inline]
    pub fn results(&self) -> &[ArchLinuxPkgInfo] {
        &self.results
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

#[derive(Deserialize, Debug)]
pub struct ArchLinuxPkgInfo {
    pub pkgname: String,
    pub repo: String,
    pub pkgver: String,
    pub pkgrel: String,
    pub pkgdesc: String,
    pub url: String,
    pub installed_size: u32,
    pub last_update: String,
}

pub async fn fetch_pkg_info(data: AppData, pkg: &str) -> anyhow::Result<Sendable> {
    let url = reqwest::Url::parse_with_params(ARCH_PKG_SEARCH_API, &[("name", pkg)])
        .with_context(|| format!("{pkg} is a invalid params"))?;

    let resp: ArchLinuxSearchResponse = data.requester.to_t(url).await?;
    if !resp.is_valid() {
        anyhow::bail!("invalid request!")
    }

    let pkg = resp
        .results()
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no result found for {pkg}"))?;

    let display = format!(
        "Name: {} \
         Repo: {} \
         Version: {}-{} \
         Description: {} \
         Upstream: {} \
         Installed Size: {} \
         Last Update: {} \
       ",
        pkg.pkgname,
        pkg.repo,
        pkg.pkgver,
        pkg.pkgrel,
        pkg.pkgdesc,
        pkg.url,
        pkg.installed_size,
        pkg.last_update
    );

    Ok(Sendable::builder().text(display).build())
}

pub async fn fetch_pkg_list(data: AppData, pkg: &str, max: usize) -> anyhow::Result<Sendable> {
    let query_by = |typ: &str| -> anyhow::Result<reqwest::Url> {
        reqwest::Url::parse_with_params(ARCH_PKG_SEARCH_API, &[(typ, pkg)])
            .with_context(|| format!("{pkg} is a invalid params"))
    };

    let req = &data.requester;
    let (exact_match, fuzzy_match) = tokio::join! {
         req.to_t::<ArchLinuxSearchResponse>(query_by("name")?),
         req.to_t::<ArchLinuxSearchResponse>(query_by("q")?),
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
