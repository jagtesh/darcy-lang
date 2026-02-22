use dslc::compile;

#[test]
fn lowers_comparisons() {
    let src = "(defn main [] (do (= 1 2) (< 1 2) (<= 1 2) (> 2 1) (>= 2 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("1i64 == 2i64"), "{}", out);
    assert!(out.contains("1i64 < 2i64"), "{}", out);
    assert!(out.contains("1i64 <= 2i64"), "{}", out);
    assert!(out.contains("2i64 > 1i64"), "{}", out);
    assert!(out.contains("2i64 >= 1i64"), "{}", out);
}

#[test]
fn lowers_bitwise_ops() {
    let src = "(defn main [] (do (& 6 3) (| 6 3)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("6i64 & 3i64"), "{}", out);
    assert!(out.contains("6i64 | 3i64"), "{}", out);
}

#[test]
fn lowers_mod() {
    let src = "(defn main [] (mod 5 2))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("5i64 % 2i64"), "{}", out);
}

#[test]
fn lowers_and_or() {
    let src = "(defn main [] (do (and true false) (or false true)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("if"), "{}", out);
    assert!(out.contains("let"), "{}", out);
}
