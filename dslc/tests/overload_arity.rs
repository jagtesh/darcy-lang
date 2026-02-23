use dslc::compile;

#[test]
fn lowers_overloads_by_arity() {
    let src = "\
        (defn foo [x:i32] x)
        (defn foo [x:i32 y:i32] (+ x y))
        (defn main [] (do (foo 1) (foo 1 2)))
    ";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn foo__arity1("), "{}", out);
    assert!(out.contains("fn foo__arity2("), "{}", out);
    assert!(out.contains("foo__arity1(1i32)"), "{}", out);
    assert!(out.contains("foo__arity2(1i32, 2i32)"), "{}", out);
}

#[test]
fn rejects_duplicate_overload_arity() {
    let src = "\
        (defn foo [x:i32] x)
        (defn foo [y:i32] y)
    ";
    let err = compile(src).expect_err("expected duplicate arity error");
    assert!(
        err.message
            .contains("duplicate function 'foo' with arity 1"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn rejects_missing_overload_arity() {
    let src = "\
        (defn foo [x:i32] x)
        (defn foo [x:i32 y:i32] (+ x y))
        (defn main [] (foo 1 2 3))
    ";
    let err = compile(src).expect_err("expected arity mismatch error");
    assert!(
        err.message
            .contains("function 'foo' has no overload with arity 3"),
        "unexpected error: {}",
        err.message
    );
}
