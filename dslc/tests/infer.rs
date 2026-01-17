use dslc::compile;

#[test]
fn infers_param_from_literal() {
    let src = "(defn add1 [x] (+ x 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn add1(x: i32) -> i32"), "{}", out);
}

#[test]
fn infers_param_from_call() {
    let src = "(defn inc [x:i32] (+ x 1)) (defn use [y] (inc y))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn use(y: i32) -> i32"), "{}", out);
}

#[test]
fn rejects_ambiguous_numeric_op() {
    let src = "(defn add [x y] (+ x y))";
    let err = compile(src).expect_err("expected type error");
    assert!(
        err.message.contains("ambiguous numeric operator types"),
        "{}",
        err.message
    );
}
