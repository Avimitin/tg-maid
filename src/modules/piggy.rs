use super::Sendable;
use crate::app::AppData;
use anyhow::Result;
use rand::Rng;
use scraper::{Html, Selector};

pub async fn get_pig_recipe(data: AppData) -> Result<Sendable> {
    let page: u32 = rand::thread_rng().gen_range(0..600);
    let url = format!("https://www.meishichina.com/YuanLiao/ZhuRou/{page}");

    let page = data.requester.get_text(&url).await?;

    // Deserialize HTML page is a heavy task, however we don't have async way to do
    // it. So what I can do is just not let the job block current thread.
    let recipe = tokio::task::block_in_place(move || -> String {
        let res = collect_pig_recipe(&page);
        match res {
            Ok(v) => {
                let choice: usize = rand::thread_rng().gen_range(0..v.len());
                format!("今天我们这样吃 piggy: {}", v[choice])
            }
            Err(e) => format!("今天没法吃 piggy 了呜呜呜: {e}"),
        }
    });

    Ok(Sendable::text(recipe))
}

fn collect_pig_recipe(page: &str) -> Result<Vec<String>> {
    let page = Html::parse_fragment(page);
    let recipe_list = Selector::parse(".plist li a div").unwrap();

    // take one
    let recipe_list = page
        .select(&recipe_list)
        .filter_map(|elem| elem.text().next())
        .map(|text| text.to_string())
        .collect::<Vec<_>>();

    if recipe_list.is_empty() {
        anyhow::bail!("Can not find any recipe for cooking piggy")
    }

    Ok(recipe_list)
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
