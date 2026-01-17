use dslc::compile;

#[test]
fn lowers_struct_constructor() {
    let src = "(defstruct Order (id i32) (qty i32)) (defn make [] (Order 1 2))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("Order { id: 1, qty: 2 }"));
}
