use std::str::FromStr;

/// Get list of value from the given environment variable by spliting the value of that variable
/// with character `,`.
pub fn get_list_from_env<T: FromStr>(key: &str) -> Vec<T> {
    std::env::var(key)
        .unwrap_or_else(|_| panic!("${key} not found in your env"))
        .split(',')
        .map(|number| {
            number.parse::<T>().unwrap_or_else(|_| {
                panic!(
                    "invalid value {number}, expect type: {}",
                    std::any::type_name::<T>()
                )
            })
        })
        .collect::<Vec<T>>()
}

pub fn env_get_var(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} not found in your env"))
}

pub fn parse_from_env<T: FromStr>(key: &str) -> T {
    env_get_var(key)
        .parse::<T>()
        .unwrap_or_else(|_| panic!("invalid value, expect type {}", std::any::type_name::<T>()))
}

macro_rules! generate_html_tags {
    ($($tag:ident),+) => {
        pub struct Html;
        impl Html {
            $(
                #[inline]
                pub fn $tag(text: impl std::fmt::Display) -> String {
                    const START: &str = concat!("<", stringify!($tag), ">");
                    const END: &str = concat!("</", stringify!($tag), ">");
                    format!("{START}{text}{END}")
                }
            )+
        }
    };
}

generate_html_tags![code, b, i, u, s, span, pre];

impl Html {
    #[inline]
    pub fn a(href: &str, text: &str) -> String {
        format!(r#"<a href="{href}">{text}</a>"#)
    }
}
