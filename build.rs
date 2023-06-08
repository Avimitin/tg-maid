use std::path;

fn main() {
    let bold = path::Path::new("./assets/NotoSansCJK-Bold.ttc");
    let thin = path::Path::new("./assets/NotoSansCJK-Light.ttc");

    assert!(bold.exists());
    assert!(thin.exists());

    println!(
        "cargo:rustc-env=SANS_LIGHT_PATH={}",
        thin.canonicalize().unwrap().display()
    );
    println!(
        "cargo:rustc-env=SANS_BOLD_PATH={}",
        bold.canonicalize().unwrap().display()
    );
}
