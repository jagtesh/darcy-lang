use dslc::compile;

#[test]
fn lowers_let_and_lambda_call() {
    let src = "(defn main [] (let [x 1 y 2] (call (fn [z] (+ z x)) y)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("let x = 1i64;"), "{}", out);
    assert!(out.contains("let y = 2i64;"), "{}", out);
    assert!(out.contains("|z| {"), "{}", out);
    assert!(out.contains(")(y)"), "{}", out);
}

#[test]
fn lowers_def_values() {
    let src = "(def counter 1) (defn main [] (+ counter 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("static counter: LazyLock<i64>"), "{}", out);
    assert!(out.contains("(*counter).clone()"), "{}", out);
}

#[test]
fn lowers_kebab_and_uppercase_def_values() {
    let src = "(def const-pi 3.142) (def PI 3.141592653589793) (defn main [] (+ const-pi PI))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("static const_pi: LazyLock<f64>"), "{}", out);
    assert!(out.contains("static PI: LazyLock<f64>"), "{}", out);
    assert!(out.contains("(*const_pi).clone()"), "{}", out);
    assert!(out.contains("(*PI).clone()"), "{}", out);
}
