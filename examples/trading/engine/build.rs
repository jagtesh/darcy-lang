use std::path::PathBuf;

fn main() {
    if std::env::var("CARGO_FEATURE_DARCY_COMPILED").is_err() {
        return;
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let darcy_root = manifest_dir.join("../darcy");
    let entry = darcy_root.join("trading/main.dsl");

    darcy_build::Builder::new(entry)
        .lib_path(darcy_root)
        .stdlib_path(darcy_stdlib::stdlib_dir())
        .compile()
        .expect("darcy compile failed");
}
