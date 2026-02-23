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
    let src = "(defenum opt (some (v i32)) (none)) (defn f [o:opt] (case o (some (v v) v)))";
    let err = compile(src).expect_err("expected non-exhaustive match");
    assert!(err.message.contains("non-exhaustive"), "{}", err.message);
}

#[test]
fn wildcard_match_ok() {
    let src = "(defenum opt (some (v i32)) (none)) (defn f [o:opt] (case o (some (v v) v) (_ 0)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("_ => 0"));
}

#[test]
fn enum_generates_display_impl() {
    let src = "(defenum state (on) (off)) (defn main [s:state] (darcy.fmt/println s))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("impl std::fmt::Display for State"), "{}", out);
    assert!(out.contains("println!(\"{}\", s)"), "{}", out);
}
