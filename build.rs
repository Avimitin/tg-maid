use std::{env, path};

fn bridge_env(key: &str) {
    let font_path =
        env::var(key).unwrap_or_else(|_| panic!("`{key}` not set, please set it to a font path"));
    let font_path = path::PathBuf::from(font_path);
    if !font_path.exists() {
        panic!("Font not found in path: {}", font_path.display());
    }
    println!(
        "cargo:rustc-env={}={}",
        key,
        font_path.canonicalize().unwrap().display()
    )
}

fn main() {
    bridge_env("QUOTE_TEXT_FONT_PATH");
    bridge_env("QUOTE_USERNAME_FONT_PATH");
}
