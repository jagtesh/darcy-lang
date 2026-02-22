use dslc::compile;

#[test]
fn struct_fields_use_lowercase_u8() {
    let src = "(defrecord pixel (x i32) (y i32) (r u8) (g u8) (b u8))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub r: u8"), "{}", out);
    assert!(out.contains("pub g: u8"), "{}", out);
    assert!(out.contains("pub b: u8"), "{}", out);
}
