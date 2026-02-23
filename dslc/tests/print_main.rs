use dslc::compile;

#[test]
fn lowers_print_and_main() {
    let src = "(defn main [] (darcy.io/dbg 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("pub fn main()"));
    assert!(out.contains("println!"));
}

#[test]
fn lowers_print_string() {
    let src = "(defn main [] (darcy.io/dbg \"hello\"))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("println!"));
    assert!(out.contains("String::from"));
}

#[test]
fn lowers_fmt_format_and_pretty() {
    let src = "(defn main [] (darcy.fmt/format 1)) (defn other [] (darcy.fmt/pretty 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("format!(\"{:?}\", 1i64)"), "{}", out);
    assert!(out.contains("format!(\"{:#?}\", 1i64)"), "{}", out);
}

#[test]
fn lowers_fmt_print() {
    let src = "(defn main [] (darcy.fmt/print (darcy.fmt/format 1)))";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("print!(\"{}\", format!(\"{:?}\", 1i64))"),
        "{}",
        out
    );
}

#[test]
fn lowers_fmt_println() {
    let src = "(defn main [] (darcy.fmt/println (darcy.fmt/format 1)))";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("println!(\"{}\", format!(\"{:?}\", 1i64))"),
        "{}",
        out
    );
}

#[test]
fn lowers_fmt_println_variadic_template() {
    let src = "(defn main [x:i64 y:i64] (darcy.fmt/println \"x={} y={}\" x y))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("println!(\"x={} y={}\", x, y)"), "{}", out);
}

#[test]
fn lowers_string_interpolation_in_println() {
    let src = "(defn main [name:string] (darcy.fmt/println \"hi ${name}\"))";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("format!(\"hi {}\", darcy_stdlib::rt::fmt_format(name))"),
        "{}",
        out
    );
    assert!(
        out.contains("println!(\"{}\", format!(\"hi {}\", darcy_stdlib::rt::fmt_format(name)))"),
        "{}",
        out
    );
}

#[test]
fn println_accepts_record_directly_via_display() {
    let src = "(defrecord user [name:string]) (defn main [u:user] (darcy.fmt/println u))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("impl std::fmt::Display for User"), "{}", out);
    assert!(out.contains("println!(\"{}\", u)"), "{}", out);
}
