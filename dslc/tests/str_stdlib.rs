use dslc::compile;

#[test]
fn core_str_len_is_empty_lowers() {
    let src = "(defn main [s:string] (darcy.string/is-empty s))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".is_empty()"), "{}", out);
}

#[test]
fn core_str_trim_lowers() {
    let src = "(defn main [s:string] (darcy.string/trim s))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".trim().to_string()"), "{}", out);
}

#[test]
fn core_str_split_join_lowers() {
    let src =
        "(defn main [s:string sep:string] (darcy.string/join (darcy.string/split s sep) \"|\"))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".split("), "{}", out);
    assert!(out.contains(".join("), "{}", out);
}
