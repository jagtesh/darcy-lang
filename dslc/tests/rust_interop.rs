use std::path::PathBuf;

use dslc::compile_with_modules;

#[test]
fn defextern_macro_expands() {
    let lib_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crates/darcy-stdlib/darcy");
    let src = "(require [darcy.rust :refer [defextern defextern-record]]) (defextern-record mnist-data \"darcy_runtime::mnist::MnistData\" [(images vec<vec<f64>>) (labels vec<vec<f64>>)]) (defextern load-edn-gz [path:string] mnist-data \"darcy_runtime::mnist::load_edn_gz\") (defn main [] (load-edn-gz \"x\"))";
    let out = compile_with_modules(&PathBuf::from("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("darcy_runtime::mnist::load_edn_gz"), "{}", out);
}
