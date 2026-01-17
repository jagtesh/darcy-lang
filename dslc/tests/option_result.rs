use dslc::compile;

#[test]
fn option_some_none_lowers() {
    let src = "(defn main [] (core.option/is-some (core.option/some 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("is_some"), "{}", out);
}

#[test]
fn result_ok_err_lowers() {
    let src = "(defn main [] (core.result/is-err (core.result/err 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("is_err"), "{}", out);
}
