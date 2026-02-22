use dslc::compile;

#[test]
fn lowers_if_without_else() {
    let src = "(defn main [b:bool] (if b 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("if b { 1i64 } else { () }"), "{}", out);
}

#[test]
fn lowers_loop_break_value() {
    let src = "(defn main [] (loop (break 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("loop { break 1i64; }"), "{}", out);
}

#[test]
fn lowers_while_break_value() {
    let src = "(defn main [b:bool] (while b (break 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("if !(b) { break; }"), "{}", out);
    assert!(out.contains("break 1i64"), "{}", out);
}

#[test]
fn lowers_for_range() {
    let src = "(defn main [] (for i (range 0 3) (darcy.io/dbg i)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("let __start"), "{}", out);
    assert!(out.contains("loop {"), "{}", out);
}

#[test]
fn lowers_for_range_incl() {
    let src = "(defn main [] (for i (range-incl 0 3) (darcy.io/dbg i)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("<="), "{}", out);
}

#[test]
fn lowers_for_for_x_y() {
    let src = "(defn main [] (for i (vec<i32> 1 2 3) (darcy.io/dbg i)))";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("for i in (Arc::new(vec![1i32, 2i32, 3i32])).iter().cloned() {"),
        "{}",
        out
    );
}

#[test]
fn lowers_set_assignment() {
    let src = "(defn main [] (do (let [x 1] (do (let! x 2) x))))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("x = 2i64"), "{}", out);
}

#[test]
fn lowers_for_float_range() {
    let src = "(defn main [] (for i (range 0.0 1.0) (darcy.io/dbg i)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("let __start = 0.0"), "{}", out);
    assert!(out.contains("let __step = 1.0"), "{}", out);
}

#[test]
fn lowers_do_sequence() {
    let src = "(defn main [] (do (darcy.io/dbg 1) (darcy.io/dbg 2)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("println!(\"{:?}\", 1i64);"), "{}", out);
    assert!(out.contains("println!(\"{:?}\", 2i64)"), "{}", out);
}

#[test]
fn lowers_and_or_short_circuit() {
    let src = "(defn main [a:bool b:bool] (and a (or b a)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("if"), "{}", out);
    assert!(out.contains("and_temp"), "{}", out);
    assert!(out.contains("or_temp"), "{}", out);
}

#[test]
fn lowers_when_form() {
    let src = "(defn main [b:bool] (when b (darcy.io/dbg 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("if b {"), "{}", out);
}
