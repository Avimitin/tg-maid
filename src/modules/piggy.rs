use anyhow::Result;
use rand::Rng;
use scraper::{Html, Selector};

fn collect_pig_recipe(page: &str) -> Result<Vec<String>> {
    let page = Html::parse_fragment(page);
    let recipe_list = Selector::parse(".plist li a div").unwrap();

    // take one
    let recipe_list = page
        .select(&recipe_list)
        .into_iter()
        .filter_map(|elem| elem.text().next())
        .map(|text| text.to_string())
        .collect::<Vec<_>>();

    if recipe_list.is_empty() {
        anyhow::bail!("Can not find any recipe for cooking piggy")
    }

    Ok(recipe_list)
}

fn collect_recipe(page: &str) -> Result<Vec<String>> {
    let page = Html::parse_fragment(page);
    let recipes = Selector::parse(".detail h2").unwrap();
    let recipes = page
        .select(&recipes)
        .filter_map(|rec| rec.text().next().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    if recipes.is_empty() {
        anyhow::bail!("Can not find any recipe")
    }

    Ok(recipes)
}

#[tokio::test]
async fn test_select_recipe() {
    let text = reqwest::get("https://www.meishichina.com/YuanLiao/ZhuRou/")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    dbg!(collect_pig_recipe(&text).unwrap());
}

#[async_trait::async_trait]
pub trait RecipeProvider {
    type Result;
    async fn get_pig_recipe(&self) -> Self::Result;
    async fn get_recipe(&self) -> Self::Result;
}

#[async_trait::async_trait]
impl RecipeProvider for crate::maid::Fetcher {
    type Result = anyhow::Result<String>;

    async fn get_pig_recipe(&self) -> Self::Result {
        let page: u32 = rand::thread_rng().gen_range(0..600);
        let url = reqwest::Url::parse(&format!(
            "https://www.meishichina.com/YuanLiao/ZhuRou/{page}"
        ))
        .unwrap();

        let page = self.fetch(url).await?;

        // Deserialize HTML page is a heavy task, however we don't have async way to do
        // it. So what I can do is just not let the job block current thread.
        let task = move || -> String {
            let res = collect_pig_recipe(&page);
            match res {
                Ok(v) => {
                    use rand::Rng;
                    let choice: usize = rand::thread_rng().gen_range(0..v.len());
                    format!("今天我们这样吃 piggy: {}", v[choice])
                }
                Err(e) => format!("今天没法吃 piggy 了呜呜呜: {e}"),
            }
        };

        Ok(tokio::task::block_in_place(task))
    }

    async fn get_recipe(&self) -> Self::Result {
        let page: u32 = rand::thread_rng().gen_range(0..=100);
        let url = reqwest::Url::parse(&format!(
            "https://home.meishichina.com/recipe-list-page-{}.html",
            page
        ))
        .unwrap();
        let page = self.fetch(url).await?;

        let task = move || -> String {
            let res = collect_recipe(&page);
            match res {
                Ok(v) => {
                    let choice: usize = rand::thread_rng().gen_range(0..v.len());
                    format!("吃{}吧！", v[choice])
                }
                Err(_) => "你不许吃了！".to_string(),
            }
        };

        Ok(tokio::task::block_in_place(task))
    }
}
