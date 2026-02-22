use std::path::PathBuf;

fn main() {
    if std::env::var("CARGO_FEATURE_DARCY_COMPILED").is_err() {
        return;
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let entry = manifest_dir.join("darcy/main.dsl");
    let lib_dir = manifest_dir.join("darcy");
    let stdlib = std::env::var("DARCY_STDLIB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| darcy_stdlib::stdlib_dir());

    darcy_build::Builder::new(entry)
        .lib_path(lib_dir)
        .stdlib_path(stdlib)
        .compile()
        .expect("darcy compile failed");
}
