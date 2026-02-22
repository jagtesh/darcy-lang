use std::path::PathBuf;

use dslc::compile_with_modules;

#[test]
fn tensor_module_compiles() {
    let lib_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crates/darcy-stdlib/darcy");
    let src = "(require [darcy.tensor :as t]) (defn main [] (t/vec-dot [1.0 2.0] [3.0 4.0]))";
    let out = compile_with_modules(&PathBuf::from("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("fn main"), "{}", out);
}
