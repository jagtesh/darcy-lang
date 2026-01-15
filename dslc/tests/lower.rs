use dslc::compile;

#[test]
fn lowers_numeric_binop() {
    let src = "(defstruct Order (qty u32) (price f64)) (defn total [o:Order] (* o.qty o.price))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub fn total"));
    assert!(out.contains("* o.price"));
}
