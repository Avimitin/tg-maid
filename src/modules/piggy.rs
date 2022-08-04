#[async_trait::async_trait]
pub trait RecipeProvider {
    type Result;
    async fn get_pig_recipe(&self) -> Self::Result;
}

#[async_trait::async_trait]
impl RecipeProvider for crate::butler::Fetcher {
    type Result = anyhow::Result<String>;

    async fn get_pig_recipe(&self) -> Self::Result {
        let page: u32 = rand::thread_rng().gen_range(0..600);
        let url = reqwest::Url::parse(&format!(
            "https://www.meishichina.com/YuanLiao/ZhuRou/{page}"
        ))
        .unwrap();

        self.fetch(url).await
    }
}
