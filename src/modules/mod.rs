mod archlinux;
mod collect;
mod currency;
mod ehentai;
mod ksyx;
mod nsfw;
mod piggy;
mod weather;

pub mod provider {
    pub use super::{
        archlinux::ArchLinuxPkgProvider, currency::CurrenciesRateProvider, ehentai::EhentaiProvider,
        nsfw::NsfwProvider, piggy::RecipeProvider, weather::WeatherProvider,
    };
}

pub mod cache {
    pub use super::{
        collect::CollectedMsgCache, currency::CurrenciesCache, ksyx::KsyxCounterCache,
    };
}

pub mod prelude {
    pub use super::provider::*;
    pub use super::cache::*;
    pub use super::collect::MsgForm;
}
