mod archlinux;
mod collect;
mod currency;
mod ehentai;
mod ksyx;
mod nsfw;
mod osu;
mod piggy;
mod weather;

pub mod provider {
    pub use super::{
        archlinux::ArchLinuxPkgProvider, currency::CurrenciesRateProvider,
        ehentai::EhentaiProvider, nsfw::NsfwProvider, piggy::RecipeProvider,
        weather::WeatherProvider,
    };
}

pub mod cache {
    pub use super::{
        collect::CollectedMsgCache, currency::CurrenciesCache, ksyx::KsyxCounterCache,
        osu::OsuLocalStorage,
    };
}

pub mod prelude {
    pub use super::cache::*;
    pub use super::collect::MsgForm;
    pub use super::provider::*;
}
