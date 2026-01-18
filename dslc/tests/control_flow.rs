use dslc::compile;

#[test]
fn lowers_if_without_else() {
    let src = "(defn main [b:bool] (if b 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("if b { 1i32 } else { () }"), "{}", out);
}

#[test]
fn lowers_loop_break_value() {
    let src = "(defn main [] (loop (break 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("loop { break 1i32; }"), "{}", out);
}

#[test]
fn lowers_while_break_value() {
    let src = "(defn main [b:bool] (while b (break 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("if !(b) { break; }"), "{}", out);
    assert!(out.contains("break 1i32"), "{}", out);
}

#[test]
fn lowers_for_range() {
    let src = "(defn main [] (for i (range 0 3) (dbg i)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("let __start"), "{}", out);
    assert!(out.contains("loop {"), "{}", out);
}

#[test]
fn lowers_for_range_incl() {
    let src = "(defn main [] (for i (range-incl 0 3) (dbg i)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("<="), "{}", out);
}
