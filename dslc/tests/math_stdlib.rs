use dslc::compile;

#[test]
fn math_exp_lowers() {
    let src = "(defn main [] (darcy.math/exp 1.0))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".exp()"), "{}", out);
}

#[test]
fn math_cmp_lowers() {
    let src = "(defn main [] (darcy.math/gt 2 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(">"), "{}", out);
}
