use std::path::PathBuf;

fn main() {
    if std::env::var("CARGO_FEATURE_DARCY_COMPILED").is_err() {
        return;
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stdlib_dir = manifest_dir.join("darcy");
    let entry = stdlib_dir.join("stdlib.dsl");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let out_file = out_dir.join("darcy_stdlib.rs");

    darcy_build::Builder::new(entry)
        .lib_path(stdlib_dir.clone())
        .stdlib_path(stdlib_dir)
        .out_file(out_file)
        .compile()
        .expect("darcy stdlib compile failed");
}
