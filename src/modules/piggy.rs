use anyhow::Result;
use rand::Rng;
use scraper::{Html, Selector};

fn collect_pig_recipe(page: &str) -> Result<Vec<String>> {
    let page = Html::parse_fragment(page);
    let recipe_list = Selector::parse("ul.on").unwrap();
    let li = Selector::parse("li a p").unwrap();

    // take one
    let ul = page
        .select(&recipe_list)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Fail to select recipe from the given HTML page"))?;

    let mut v = Vec::new();
    for elem in ul.select(&li) {
        if let Some(recipe) = elem.text().next() {
            v.push(recipe.to_string());
        }
    }

    if v.is_empty() {
        anyhow::bail!("Can not find any recipe for cooking piggy")
    }

    Ok(v)
}

fn collect_recipe(page: &str) -> Result<Vec<String>> {
    let page = Html::parse_fragment(page);
    let recipes = Selector::parse(".detail").unwrap();
    let recipe_title = Selector::parse("h2").unwrap();
    let recipes = page
        .select(&recipes)
        .next()
        .ok_or_else(|| anyhow::anyhow!("Fail to select recipe"))?;

    let mut v = Vec::new();
    for elem in recipes.select(&recipe_title) {
        if let Some(text) = elem.text().next() {
            v.push(text.to_string());
        }
    }

    if v.is_empty() {
        anyhow::bail!("Can not find any recipe")
    }

    Ok(v)
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
        let page: u32 = rand::random::<u32>() % 1000;
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
                    use rand::Rng;
                    let choice: usize = rand::thread_rng().gen_range(0..v.len());
                    format!("今天吃{}", v[choice])
                }
                Err(_) => "今天不许吃饭！".to_string(),
            }
        };

        Ok(tokio::task::block_in_place(task))
    }
}
