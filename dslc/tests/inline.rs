use dslc::compile;

#[test]
fn inline_expands_in_place() {
    let src = "(defin inc [x] (+ x 1)) (defn main [y:i32] (inc y))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("y + 1i32"), "{}", out);
}

#[test]
fn inline_can_capture_caller_vars() {
    let src = "(defin add-x [y] (+ x y)) (defn main [x:i32 y:i32] (add-x y))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("x + y"), "{}", out);
}

#[test]
fn recursive_inline_is_rejected() {
    let src = "(defin looped [x] (looped x)) (defn main [] (looped 1))";
    let err = compile(src).expect_err("recursive inline should fail");
    assert!(err.message.contains("recursive defin is not allowed"));
}
