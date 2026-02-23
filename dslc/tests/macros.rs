use dslc::compile;

#[test]
fn defmacro_expands_list() {
    let src = "(defmacro twice [x] (list '+ x x)) (defn main [] (twice 2))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("2i64 + 2i64"), "{}", out);
}

#[test]
fn defmacro_expands_quote_reader() {
    let src = "(defmacro lit [] '(+ 1 2)) (defn main [] (lit))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("1i64 + 2i64"), "{}", out);
}

#[test]
fn defmacro_expands_syntax_quote_unquote() {
    let src = "(defmacro wrap [x] `(let [v# ~x] v#)) (defn main [] (wrap 3))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("let"), "{}", out);
    assert!(out.contains("3i64"), "{}", out);
}

#[test]
fn defmacro_unquote_splicing() {
    let src = "(defmacro mk [xs] `(list ~@xs)) (defn main [] (mk [1 2 3]))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("Arc::new(vec![1i64, 2i64, 3i64])"), "{}", out);
}

#[test]
fn metadata_is_ignored() {
    let src = "^:test (defn main [] 1)";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn main"), "{}", out);
}

#[test]
fn thread_first_macro_expands() {
    let src = "(defn pair [x y] y) (defn main [] (-> 1 (pair 2)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("(1i64, 2i64)"), "{}", out);
}

#[test]
fn thread_last_macro_expands() {
    let src = "(defn pair [x y] y) (defn main [] (->> 1 (pair 2)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("(2i64, 1i64)"), "{}", out);
}
