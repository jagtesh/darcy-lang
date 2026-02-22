use dslc::compile;

#[test]
fn option_some_none_lowers() {
    let src = "(defn main [] (darcy.option/is-some (darcy.option/some 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("is_some"), "{}", out);
}

#[test]
fn option_none_is_none_is_typed() {
    let src = "(defn main [] (darcy.option/is-none (darcy.option/none)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("Option::<()>::None"), "{}", out);
}

#[test]
fn result_ok_err_lowers() {
    let src = "(defn main [] (darcy.result/is-err (darcy.result/err 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("is_err"), "{}", out);
}

#[test]
fn result_ok_is_ok_is_typed() {
    let src = "(defn main [] (darcy.result/is-ok (darcy.result/ok 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("Result::<i64, ()>::Ok"), "{}", out);
}

#[test]
fn option_unwrap_lowers() {
    let src = "(defn main [] (darcy.option/unwrap (darcy.option/some 42)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".unwrap()"), "{}", out);
}

#[test]
fn option_unwrap_or_lowers() {
    let src = "(defn main [] (darcy.option/unwrap-or (darcy.option/none) 0))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".unwrap_or("), "{}", out);
}

#[test]
fn result_unwrap_lowers() {
    let src = "(defn main [] (darcy.result/unwrap (darcy.result/ok 42)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".unwrap()"), "{}", out);
}

#[test]
fn result_unwrap_or_lowers() {
    let src = "(defn main [] (darcy.result/unwrap-or (darcy.result/err 1) 0))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".unwrap_or("), "{}", out);
}
