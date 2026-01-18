use dslc::compile;

#[test]
fn lowers_print_and_main() {
    let src = "(defn main [] (dbg 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub fn main()"));
    assert!(out.contains("println!"));
}

#[test]
fn lowers_print_string() {
    let src = "(defn main [] (dbg \"hello\"))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("println!"));
    assert!(out.contains("String::from"));
}

#[test]
fn lowers_fmt_format_and_pretty() {
    let src = "(defn main [] (core.fmt/format 1)) (defn other [] (core.fmt/pretty 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("format!(\"{:?}\", 1i32)"), "{}", out);
    assert!(out.contains("format!(\"{:#?}\", 1i32)"), "{}", out);
}

#[test]
fn lowers_fmt_print() {
    let src = "(defn main [] (core.fmt/dbg 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("println!"), "{}", out);
}

#[test]
fn lowers_fmt_println_with_format() {
    let src = "(defn main [] (core.fmt/println \"x={}\" 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("println!(\"x={}\", 1i32)"), "{}", out);
}

#[test]
fn lowers_fmt_print_with_escape() {
    let src = "(defn main [] (core.fmt/print \"x={}\\n\" 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("print!(\"x={}"), "{}", out);
    assert!(out.contains(", 1i32)"), "{}", out);
}
