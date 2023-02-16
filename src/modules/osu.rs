use async_trait::async_trait;
use osu_api::api::*;
use redis::{aio::ConnectionManager, AsyncCommands};

#[async_trait]
impl osu_api::api::OsuApiRequester for crate::maid::Fetcher {
    async fn get_user_recent<'k, 'u>(
        &self,
        param: GetUserRecentProp<'k, 'u>,
    ) -> Result<Vec<GetUserRecentResp>, Error> {
        self.c.get_user_recent(param).await
    }

    async fn get_beatmaps<'u, 'k>(
        &self,
        param: GetBeatmapsProps<'u, 'k>,
    ) -> Result<Vec<GetBeatmapsResp>, Error> {
        self.c.get_beatmaps(param).await
    }
}

#[async_trait]
pub trait OsuLocalStorage {
    type Error;

    async fn get_user_osu_id(&mut self, tg_id: u64) -> Result<Option<u64>, Self::Error>;
    async fn register(&mut self, tg_id: u64, osu_id: u64) -> Result<(), Self::Error>;
}

#[async_trait]
impl OsuLocalStorage for ConnectionManager {
    type Error = redis::RedisError;

    async fn get_user_osu_id(&mut self, tg_id: u64) -> Result<Option<u64>, Self::Error> {
        let osu_id: Option<u64> = self.get(format!("tg-osu-user-map-{tg_id}")).await?;
        Ok(osu_id)
    }

    async fn register(&mut self, tg_id: u64, osu_id: u64) -> Result<(), Self::Error> {
        self.set(format!("tg-osu-user-map-{tg_id}"), osu_id).await?;
        Ok(())
    }
}
