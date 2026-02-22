use dslc::compile;

#[test]
fn core_clone_lowers() {
    let src = "(defn main [v:vec<i32>] (darcy.core/clone v))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".clone()"), "{}", out);
}
