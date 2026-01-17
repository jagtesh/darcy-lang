use dslc::compile;

#[test]
fn core_vec_len_lowers() {
    let src = "(defn main [xs:vec<i32>] (core.vec/len xs))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".len()"), "{}", out);
}

#[test]
fn core_vec_is_empty_lowers() {
    let src = "(defn main [xs:vec<i32>] (core.vec/is-empty xs))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".is_empty()"), "{}", out);
}

#[test]
fn core_num_min_max_clamp_lowers() {
    let src = "(defn main [x:i32 y:i32 z:i32] (core.num/clamp (core.num/max x y) 0 z))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".max("), "{}", out);
    assert!(out.contains(".clamp("), "{}", out);
}

#[test]
fn core_num_abs_lowers() {
    let src = "(defn main [x:i32] (core.num/abs x))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".abs()"), "{}", out);
}
