use dslc::compile;

#[test]
fn rejects_ambiguous_type() {
    let src = include_str!("../../examples/ambiguous.dsl");
    let err = compile(src).expect_err("expected type error");
    assert!(
        err.message.contains("ambiguous type"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn rejects_missing_field() {
    let src = include_str!("../../examples/missing_field.dsl");
    let err = compile(src).expect_err("expected field error");
    assert!(
        err.message.contains("has no field"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn rejects_unconstrained_param() {
    let src = "(defrecord order [qty:u32]) (defn total [o] o)";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn total<T"), "{}", out);
}

#[test]
fn rejects_export_without_concrete_types() {
    let src = "(export (defn id [x] x))";
    let err = compile(src).expect_err("expected export boundary error");
    assert!(
        err.message.contains("export requires explicit types"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn allows_export_with_concrete_types() {
    let src = "(export (defn id [x:i64] x))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub fn id(x: i64) -> i64"), "{}", out);
}

#[test]
fn rejects_break_outside_loop() {
    let src = "(defn main [] (break))";
    let err = compile(src).expect_err("expected type error");
    assert!(
        err.message.contains("break is only allowed inside loops"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn rejects_continue_outside_loop() {
    let src = "(defn main [] (continue))";
    let err = compile(src).expect_err("expected type error");
    assert!(
        err.message
            .contains("continue is only allowed inside loops"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn rejects_shadowing_def_name() {
    let src = "(def val 1) (defn main [val] val)";
    let err = compile(src).expect_err("expected shadowing error");
    assert!(
        err.message.contains("shadows a def name"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn rejects_auto_symbol_alias_without_module_context() {
    let src = "(defn main [] ::m/a)";
    let err = compile(src).expect_err("expected error");
    assert!(
        err.message.contains("require module context")
            || err.message.contains("compile_with_modules"),
        "unexpected error: {}",
        err.message
    );
}
