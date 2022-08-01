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

#[async_trait::async_trait]
pub trait ArchLinuxPacman {
    async fn exact_match(&self, pkg: &str) -> anyhow::Result<()>;
    async fn fuzzy_match(&self, pkg: &str);
}

use anyhow::Context;

use crate::modules::req;

const SEARCH_BASE_URL: &str = "https://www.archlinux.org/packages/search/json";

impl req::Client {
    pub async fn get_archpkg_info(&self, pkg: &str) -> anyhow::Result<PackageInfo> {
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

    pub async fn search_archpkg(
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

#[tokio::test]
async fn test_exact_match() {
    let client = req::Client::new();
    let info = client
        .get_archpkg_info("neovim")
        .await
        .expect("request should success");
    println!("{}", info);
}
