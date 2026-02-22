use dslc::compile;

#[test]
fn lowers_numeric_binop() {
    let src = "(defrecord order (qty u32) (price f64)) (defn total [o:order] (* o.qty o.price))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn total"));
    assert!(out.contains("* o.price"));
}

#[test]
fn lowers_abs_with_typed_negative_literal() {
    let src = "(defn main [] (darcy.math/abs -42))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("(-42i64).abs()"), "{}", out);
}
