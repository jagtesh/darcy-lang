use dslc::compile;

#[test]
fn lowers_union_and_match() {
    let src = include_str!("../../examples/union.dsl");
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub enum Shape"));
    assert!(out.contains("match"));
    assert!(out.contains("Shape::Circle"));
}

#[test]
fn rejects_non_exhaustive_match() {
    let src = "(defunion Opt (Some (v i32)) (None)) (defn f [o:Opt] (match o (Some (v v) v)))";
    let err = compile(src).expect_err("expected non-exhaustive match");
    assert!(err.message.contains("non-exhaustive"), "{}", err.message);
}

#[test]
fn wildcard_match_ok() {
    let src = "(defunion Opt (Some (v i32)) (None)) (defn f [o:Opt] (match o (Some (v v) v) (_ 0)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("_ => 0"));
}
