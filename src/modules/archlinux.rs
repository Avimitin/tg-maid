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
    pub async fn exact_match(&self, pkg: &str) -> anyhow::Result<PackageInfo> {
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
}

#[tokio::test]
async fn test_exact_match() {
    let client = req::Client::new();
    let info = client
        .exact_match("neovim")
        .await
        .expect("request should success");
    println!("{}", info);
}
