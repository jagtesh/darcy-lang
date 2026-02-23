use dslc::compile;

#[test]
fn lowers_function_call() {
    let src = "(defn total-prices [x:i32] (+ x 1)) (defn main [] (darcy.io/dbg (total-prices 4)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("total_prices(4i32)"), "{}", out);
}

#[test]
fn lowers_predicate_function_call() {
    let src = "(defn empty? [x] true) (defn main [] (empty? 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("empty_q"), "{}", out);
}
