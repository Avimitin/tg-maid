use std::str::FromStr;

/// Get list of value from the given environment variable by spliting the value of that variable
/// with character `,`.
pub fn get_list_from_var<T: FromStr>(key: &str) -> Vec<T> {
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
