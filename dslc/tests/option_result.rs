use dslc::compile;

#[test]
fn option_some_none_lowers() {
    let src = "(defn main [] (core.option/is-some (core.option/some 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("is_some"), "{}", out);
}

#[test]
fn option_none_is_none_is_typed() {
    let src = "(defn main [] (core.option/is-none (core.option/none)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("Option::<()>::None"), "{}", out);
}

#[test]
fn result_ok_err_lowers() {
    let src = "(defn main [] (core.result/is-err (core.result/err 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("is_err"), "{}", out);
}

#[test]
fn result_ok_is_ok_is_typed() {
    let src = "(defn main [] (core.result/is-ok (core.result/ok 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("Result::<i32, ()>::Ok"), "{}", out);
}
