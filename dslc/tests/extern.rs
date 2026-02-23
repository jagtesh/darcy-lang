use dslc::compile;

#[test]
fn lowers_extern_overrides() {
    let src = "(extern \"OddType\" (defrecord odd-type [v:i32])) (extern \"odd_fn\" (defn odd-fn [x:odd-type] i32)) (defn main [] (darcy.io/dbg (odd-fn (odd-type 1))))";
    let out = compile(src).expect("compile ok");
    assert!(!out.contains("pub struct OddType"));
    assert!(!out.contains("pub fn odd_fn"));
    assert!(out.contains("odd_fn"));
    assert!(out.contains("OddType"));
}
