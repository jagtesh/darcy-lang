use std::path::PathBuf;

use dslc::{compile, compile_with_modules};

#[test]
fn hashmap_literal_lowers() {
    let src = "(defn main [] (darcy.hash-map/new [:a 1] [:b 2]))";
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
fn set_literals_lower() {
    let src = "(defn main [] (do (hashset 1 2) #{3 4}))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("HashSet"), "{}", out);
    assert!(out.contains("insert"), "{}", out);
}

#[test]
fn set_empty_with_annotation() {
    let src = "(defn main [] (set<i32>))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("HashSet"), "{}", out);
    assert!(out.contains("HashSet::<i32>::new"), "{}", out);
}

#[test]
fn btreemap_literal_lowers() {
    let src = "(defn main [] (darcy.btree-map/new [:a 1]))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("BTreeMap"), "{}", out);
}

#[test]
fn hashmap_get_contains_lowers() {
    let src = "(defn main [m:hash-map<string,i32>] (darcy.hash-map/contains m :a))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("contains_key"), "{}", out);
}

#[test]
fn hashmap_empty_with_annotation() {
    let src = "(defn main [] (darcy.hash-map/new<string,i32>))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("HashMap::new"), "{}", out);
}

#[test]
fn hashmap_alias_new_lowers() {
    let root = PathBuf::from("main.dsl");
    let src = "(require [darcy.hash-map :as hm]) (defn main [] (hm/new [:a 1]))";
    let out = compile_with_modules(&root, src, &[])
        .expect("compile ok")
        .rust;
    assert!(out.contains("HashMap"), "{}", out);
}
