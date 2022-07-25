use anyhow::Result;
use scraper::{Html, Selector};

pub fn collect_recipe(page: &str) -> Result<Vec<String>> {
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

#[tokio::test]
async fn test_select_recipe() {
    let text = reqwest::get("https://www.meishichina.com/YuanLiao/ZhuRou/")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    dbg!(collect_recipe(&text).unwrap());
}
