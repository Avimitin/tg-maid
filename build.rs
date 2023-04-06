use fontconfig::Fontconfig;

fn main() {
    let fc = Fontconfig::new().expect("fail to initialize font config");
    let sans_light = fc
        .find("Noto Sans CJK SC", Some("style=light"))
        .expect("Font Noto Sans CJK SC Light not found");
    let sans_bold = fc
        .find("Noto Sans CJK SC", Some("style=bold"))
        .expect("Font Noto Sans CJK SC Bold not found");

    println!(
        "cargo:rustc-env=SANS_LIGHT_PATH={}",
        sans_light.path.as_os_str().to_str().unwrap()
    );

    println!(
        "cargo:rustc-env=SANS_BOLD_PATH={}",
        sans_bold.path.as_os_str().to_str().unwrap()
    );
}
