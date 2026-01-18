use dslc::compile;

#[test]
fn broadcasts_vector_scalar() {
    let src = "(defn scale [] (* [1 2 3] 2))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("map(|__x| __x * 2i32)"), "{}", out);
}

#[test]
fn broadcasts_struct_field_access() {
    let src = "(defstruct order (qty u32)) (defn qtys [os:vec<order>] os.qty)";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("map(|__x| __x.qty)"));
}

#[test]
fn rejects_vector_scalar_mismatch() {
    let src = "(defn scale [] (* [1 2 3] 2.0))";
    let err = compile(src).expect_err("expected type error");
    assert!(err.message.contains("vector-scalar"), "{}", err.message);
}

#[test]
fn vec_get_lowers() {
    let src = "(defn main [v:vec<i32>] (core.vec/get v 0))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("as usize"), "{}", out);
    assert!(out.contains(".clone()"), "{}", out);
}

#[test]
fn vec_set_lowers() {
    let src = "(defn main [v:vec<i32>] (core.vec/set v 0 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("let mut __v"), "{}", out);
    assert!(out.contains("as usize"), "{}", out);
}
