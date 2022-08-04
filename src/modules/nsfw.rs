use async_trait::async_trait;

/// NsfwContentFetcher require two type of output. One should return
/// Anime Waifu uwu, another one should return porn photograph.
///
/// The default implementation use Konachan as the R18 Anime image source
/// and Taobao image comment as photograph source.
#[async_trait]
pub trait NsfwContentFetcher {
    type AnimeOutput;
    type PhotographOutput;

    async fn get_anime_image(&self) -> Self::AnimeOutput;
    async fn get_photograph(&self) -> Self::PhotographOutput;
}
