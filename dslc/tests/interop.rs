use dslc::compile;

#[test]
fn lowers_method_call_dot_form() {
    let src = "(defn main [v:vec<i32>] (let [n:usize (. v len)] n))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("((v).clone()).len()"), "{}", out);
}

#[test]
fn lowers_method_call_prefix_form() {
    let src = "(defn main [v:vec<i32>] (let [n:usize (.len v)] n))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("((v).clone()).len()"), "{}", out);
}

#[test]
fn lowers_dot_field_access() {
    let src = "(defrecord box [value:i32]) (defn main [b:box] b.value)";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("b.value"), "{}", out);
}
