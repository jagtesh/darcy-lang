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
    dir.push(format!(
        "dslc_module_{}_{}_{}",
        tag,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn use_with_alias_prefix() {
    let root = temp_root("alias");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(require [math :as m]) (defn main [] (m/inc 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("fn main"));
}

#[test]
fn open_imports_all() {
    let root = temp_root("open");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(require [math :refer :all]) (defn main [] (inc 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("fn main"));
}

#[test]
fn use_only_imports_selected() {
    let root = temp_root("only");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(require [math :refer [inc]]) (defn main [] (inc 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("fn main"));
}

#[test]
fn use_only_imports_kebab_case_defs() {
    let root = temp_root("only_kebab_def");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(def const-pi 3.14159)").expect("write module");

    let src = "(require [math :refer [const-pi]]) (defn main [] const-pi)";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("fn main"), "{}", out);
}

#[test]
fn use_only_rejects_missing_names() {
    let root = temp_root("only_err");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(require [math :refer [inc]]) (defn main [] (dec 1))";
    let err =
        compile_with_modules(&root.join("main.dsl"), src, &[lib_dir]).expect_err("expected error");
    assert!(err.message.contains("unresolved name 'dec'"));
}

#[test]
fn dotted_module_prefix() {
    let root = temp_root("dotted");
    let lib_dir = root.join("lib");
    let acme_dir = lib_dir.join("acme");
    fs::create_dir_all(&acme_dir).expect("create acme dir");
    fs::write(acme_dir.join("io.dsl"), "(defn echo [x:i32] x)").expect("write module");

    let src = "(require [acme.io]) (defn main [] (acme.io/echo 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("fn main"));
}

#[test]
fn std_io_dbg_builtin_module() {
    let root = temp_root("std_io");
    let src = "(require [darcy.io]) (defn main [] (darcy.io/dbg 1))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[])
        .expect("compile ok")
        .rust;
    assert!(out.contains("println!"));
}

#[test]
fn open_core_fmt_print() {
    let root = temp_root("core_fmt");
    let src = "(require [darcy.fmt :refer [print]]) (defn main [] (print \"hi\"))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[])
        .expect("compile ok")
        .rust;
    assert!(out.contains("print!"), "{}", out);
}

#[test]
fn require_multiple_specs() {
    let root = temp_root("multi");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");
    fs::write(lib_dir.join("util.dsl"), "(defn dec [x:i32] (- x 1))").expect("write module");

    let src = "(require [math :as m] [util :as u]) (defn main [] (+ (m/inc 1) (u/dec 2)))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(out.contains("fn main"));
}

#[test]
fn prelude_println_macro_available_without_require() {
    let root = temp_root("prelude_println");
    let src = "(defn main [] (println \"n={}\" 7))";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[])
        .expect("compile ok")
        .rust;
    assert!(out.contains("println!(\"n={}\", 7i64)"), "{}", out);
}

#[test]
fn auto_symbol_resolves_to_current_module() {
    let root = temp_root("auto_sym_current");
    let src = "(defn main [] ::a)";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[])
        .expect("compile ok")
        .rust;
    assert!(
        out.contains("darcy_stdlib::rt::symbol(\":main/a\")"),
        "{}",
        out
    );
}

#[test]
fn auto_symbol_resolves_alias_namespace() {
    let root = temp_root("auto_sym_alias");
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).expect("create lib dir");
    fs::write(lib_dir.join("math.dsl"), "(defn inc [x:i32] (+ x 1))").expect("write module");

    let src = "(require [math :as m]) (defn main [] ::m/a)";
    let out = compile_with_modules(&root.join("main.dsl"), src, &[lib_dir])
        .expect("compile ok")
        .rust;
    assert!(
        out.contains("darcy_stdlib::rt::symbol(\":math/a\")"),
        "{}",
        out
    );
}

#[test]
fn auto_symbol_unknown_alias_errors() {
    let root = temp_root("auto_sym_bad_alias");
    let src = "(defn main [] ::missing/a)";
    let err = compile_with_modules(&root.join("main.dsl"), src, &[]).expect_err("expected error");
    assert!(
        err.message.contains("unknown module 'missing'"),
        "{}",
        err.message
    );
}
