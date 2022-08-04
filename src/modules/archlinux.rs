// =================== Types ==========================
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchResponse {
    valid: bool,
    results: Vec<PackageInfo>,
}

#[derive(Deserialize, Debug)]
pub struct PackageInfo {
    pkgname: String,
    repo: String,
    pkgver: String,
    pkgrel: String,
    pkgdesc: String,
    url: String,
    installed_size: u32,
    last_update: String,
}

impl std::fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {}
Repo: {}
Version: {}-{}
Description: {}
Upstream: {}
Installed Size: {}
Last Update: {}
",
            self.pkgname,
            self.repo,
            self.pkgver,
            self.pkgrel,
            self.pkgdesc,
            self.url,
            self.installed_size,
            self.last_update
        )
    }
}

/// Types that implement ArchLinuxPacman trait should have two method:
/// `pacman -Si` and `pacman -Ss`.
#[async_trait::async_trait]
pub trait ArchLinuxPacman {
    /// Customize search output type
    type SearchOutput;
    /// Act like `pacman -Ss`, and with `max` limit the output items
    async fn search_pkg(&self, pkg: &str, max: usize) -> Self::SearchOutput;
    /// Act like `pacman -Si`
    async fn get_pkg_info(&self, pkg: &str) -> anyhow::Result<PackageInfo>;
}

use crate::butler::Fetcher;
use anyhow::Context;

const SEARCH_BASE_URL: &str = "https://www.archlinux.org/packages/search/json";

#[async_trait::async_trait]
impl ArchLinuxPacman for Fetcher {
    async fn get_pkg_info(&self, pkg: &str) -> anyhow::Result<PackageInfo> {
        let url = reqwest::Url::parse_with_params(SEARCH_BASE_URL, &[("name", pkg)])
            .with_context(|| format!("{pkg} is a invalid params"))?;

        let resp = self.to_t::<SearchResponse>(url).await?;
        if !resp.valid {
            anyhow::bail!("invalid request!")
        }

        resp.results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no result found for {pkg}"))
    }

    type SearchOutput = anyhow::Result<(Option<String>, Vec<String>)>;

    async fn search_pkg(
        &self,
        pkg: &str,
        max: usize,
    ) -> anyhow::Result<(Option<String>, Vec<String>)> {
        let exact_url = reqwest::Url::parse_with_params(SEARCH_BASE_URL, &[("name", pkg)])
            .with_context(|| format!("{pkg} is a invalid params"))?;
        let regex_url = reqwest::Url::parse_with_params(SEARCH_BASE_URL, &[("q", pkg)])
            .with_context(|| format!("{pkg} is a invalid params"))?;

        let (exact, regex) = tokio::join!(
            self.to_t::<SearchResponse>(exact_url),
            self.to_t::<SearchResponse>(regex_url),
        );

        let (exact, regex) = (exact?, regex?);

        if !exact.valid || !regex.valid {
            anyhow::bail!("invalid request!")
        }

        let results = regex
            .results
            .into_iter()
            .take(max)
            .map(|pkg| format!("<b>{}/{}</b>\n    {}", pkg.repo, pkg.pkgname, pkg.pkgdesc))
            .collect();

        if exact.results.is_empty() {
            Ok((None, results))
        } else {
            let exact = &exact.results[0];
            let exact = format!("{}/{}\n\t{}", exact.repo, exact.pkgname, exact.pkgdesc);
            Ok((Some(exact), results))
        }
    }
}
