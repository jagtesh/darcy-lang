use dslc::compile;

#[test]
fn lowers_function_call() {
    let src = "(defn total_prices [x:i32] (+ x 1)) (defn main [] (print (total_prices 4)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("total_prices(4)"));
}
