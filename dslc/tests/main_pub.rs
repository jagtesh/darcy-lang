use dslc::compile;

#[test]
fn main_is_public_without_defpub() {
    let src = "(defn main [] 42)";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub fn main("), "{}", out);
}
