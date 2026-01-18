use dslc::compile;

#[test]
fn lowers_struct_constructor() {
    let src = "(defrecord order (id i32) (qty i32)) (defn make [] (order 1 2))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("Order { id: 1i32, qty: 2i32 }"), "{}", out);
}
