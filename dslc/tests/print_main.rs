use dslc::compile;

#[test]
fn lowers_print_and_main() {
    let src = "(defn main [] (print 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub fn main()"));
    assert!(out.contains("println!"));
}

#[test]
fn lowers_print_string() {
    let src = "(defn main [] (print \"hello\"))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("println!"));
    assert!(out.contains("String::from"));
}
