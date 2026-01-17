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
