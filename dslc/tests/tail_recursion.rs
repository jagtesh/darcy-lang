use dslc::compile;

#[test]
fn lowers_self_tail_recursion_to_loop() {
    let src = "(defn sum-down [n:i64 acc:i64] (if (= n 0) acc (sum-down (- n 1) (+ acc n))))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("loop {"), "{}", out);
    assert!(out.contains("__tco_arg_0"), "{}", out);
    assert!(out.contains("continue;"), "{}", out);
}

#[test]
fn non_tail_recursion_is_not_rewritten() {
    let src = "(defn fact [n:i64] (if (= n 0) 1 (* n (fact (- n 1)))))";
    let out = compile(src).expect("compile ok");
    assert!(!out.contains("__tco_arg_0"), "{}", out);
}
