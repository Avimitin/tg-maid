mod archlinux;
mod collect;
mod currency;
mod ehentai;
mod ksyx;
mod nsfw;
mod piggy;
mod scraper;
mod weather;

pub mod request {
    pub use super::{
        archlinux::ArchLinuxPacman, currency::CurrenciesFetcher, ehentai::EhentaiFetcher,
        nsfw::NsfwContentFetcher, piggy::RecipeProvider, weather::WeatherReporter,
    };
}

pub mod cache {
    pub use super::{
        collect::CollectedMsgCache, currency::CurrenciesCache, ksyx::KsyxCounterCache,
    };
}

pub mod prelude {
    pub use super::request::*;
    pub use super::cache::*;
    pub use super::collect::MsgForm;
}
