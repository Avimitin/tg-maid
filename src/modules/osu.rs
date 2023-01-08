use paste::paste;

#[derive(Debug)]
pub struct OsuApi {
    key: String,
}

macro_rules! impl_api_endpoint {
    (
      $($endpoint:ident;)+
    ) => {
        paste! {
            impl OsuApi {
                $(
                    const [<API_$endpoint:upper>]: &'static str = concat!("https://osu.ppy.sh/api/", stringify!($endpoint));
                )+
            }
        }
    };
}

impl_api_endpoint! {
    get_beatmaps;
    get_user;
    get_scores;
    get_user_best;
    get_user_recent;
}
