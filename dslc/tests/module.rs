use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use dslc::compile_with_modules;

fn temp_root(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    dir.push(format!("dslc_module_{}_{}_{}", tag, std::process::id(), nanos));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn use_with_alias_prefix() {
    let root = temp_root("alias");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(use \"math\" :as m) (defn main [] (m/inc 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir]).expect("compile ok");
    assert!(out.contains("fn main"));
}

#[test]
fn open_imports_all() {
    let root = temp_root("open");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(open \"math\") (defn main [] (inc 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir]).expect("compile ok");
    assert!(out.contains("fn main"));
}

#[test]
fn use_only_imports_selected() {
    let root = temp_root("only");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(use \"math\" :only (inc)) (defn main [] (inc 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir]).expect("compile ok");
    assert!(out.contains("fn main"));
}

#[test]
fn use_only_rejects_missing_names() {
    let root = temp_root("only_err");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(use \"math\" :only (inc)) (defn main [] (dec 1))";
    let err = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir]).expect_err("expected error");
    assert!(err.message.contains("unresolved name 'dec'"));
}

#[test]
fn dotted_module_prefix() {
    let root = temp_root("dotted");
    let lib_dir = root.join("lib");
    let std_dir = lib_dir.join("std");
    fs::create_dir_all(&std_dir).expect("create std dir");
    fs::write(std_dir.join("io.dsl"), "(defn echo [x:i32] x)").expect("write module");

    let src = "(use \"std/io\") (defn main [] (std.io/echo 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir]).expect("compile ok");
    assert!(out.contains("fn main"));
}
