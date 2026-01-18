use std::path::PathBuf;

use dslc::{compile, compile_with_modules};

#[test]
fn hashmap_literal_lowers() {
    let src = "(defn main [] (core.hashmap/new (\"a\" 1) (\"b\" 2)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("HashMap"), "{}", out);
    assert!(out.contains("insert"), "{}", out);
}

#[test]
fn hashmap_brace_literal_lowers() {
    let src = "(defn main [] {:a 1 :b 2})";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("HashMap"), "{}", out);
    assert!(out.contains("insert"), "{}", out);
}

#[test]
fn btreemap_literal_lowers() {
    let src = "(defn main [] (core.btreemap/new (\"a\" 1)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("BTreeMap"), "{}", out);
}

#[test]
fn hashmap_get_contains_lowers() {
    let src = "(defn main [m:hashmap<string,i32>] (core.hashmap/contains m \"a\"))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("contains_key"), "{}", out);
}

#[test]
fn hashmap_empty_with_annotation() {
    let src = "(defn main [] (core.hashmap/new<string,i32>))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("HashMap::new"), "{}", out);
}

#[test]
fn hashmap_alias_new_lowers() {
    let root = PathBuf::from("main.dsl");
    let src = "(use core.hashmap :as hm) (defn main [] (hm/new (\"a\" 1)))";
    let out = compile_with_modules(&root, src, &[]).expect("compile ok");
    assert!(out.contains("HashMap"), "{}", out);
}
