use dslc::compile;

#[test]
fn infers_param_from_literal() {
    let src = "(defn add1 [x] (+ x 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn add1<T"), "{}", out);
    assert!(out.contains("darcy_stdlib::rt::FromInt"), "{}", out);
}

#[test]
fn infers_param_from_call() {
    let src = "(defn inc [x:i32] (+ x 1)) (defn apply-one [y] (inc y))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn apply_one(y: i32) -> i32"), "{}", out);
}

#[test]
fn defaults_ambiguous_numeric_op() {
    let src = "(defn add [x y] (+ x y))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn add<T"), "{}", out);
    assert!(out.contains("std::ops::Add"), "{}", out);
}

#[test]
fn lowers_generic_identity() {
    let src = "(defn id [x] x) (defn main [] (id 1))";
    let out = compile(src).expect("compile ok");
    assert!(!out.contains("fn id<T"), "{}", out);
    assert!(out.contains("fn id__spec_"), "{}", out);
    assert!(out.contains("id__spec_"), "{}", out);
}

#[test]
fn type_ascription_constrains_literal() {
    let src = "(defn main [] (type 1 i32))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("1i32"), "{}", out);
}

#[test]
fn generic_clone_adds_bound() {
    let src = "(defn dup [x] (darcy.core/clone x))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn dup<T"), "{}", out);
    assert!(out.contains(": Clone"), "{}", out);
}

#[test]
fn generic_len_adds_bound() {
    let src = "(defn size [x] (. x len))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn size<T"), "{}", out);
    assert!(out.contains(": __DarcyLen"), "{}", out);
    assert!(out.contains("-> usize"), "{}", out);
}
