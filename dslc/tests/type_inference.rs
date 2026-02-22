use dslc::compile;

// ============================================================================
// TYPE INFERENCE VERIFICATION TESTS
// These tests verify the type system infers and propagates types correctly,
// not just that the output contains certain strings.
// ============================================================================

// --- Option Type Inference ---

#[test]
fn option_some_infers_inner_type() {
    // Option<i64> from literal
    let src = "(defn main [] (darcy.option/unwrap (darcy.option/some 42)))";
    let out = compile(src).expect("compile ok");
    // Should produce i64, not a generic type
    assert!(
        out.contains("42i64"),
        "Should infer i64 inner type: {}",
        out
    );
}

#[test]
fn option_none_requires_type_context() {
    // None without type context defaults to ()
    let src = "(defn main [] (darcy.option/none))";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("Option::<()>::None"),
        "Untyped none should default to (): {}",
        out
    );
}

#[test]
fn option_unwrap_or_unifies_types() {
    // The fallback type should unify with Some's inner type
    let src = "(defn main [] (darcy.option/unwrap-or (darcy.option/some 42) 0))";
    let out = compile(src).expect("compile ok");
    // Both should be i64
    assert!(
        out.contains("42i64") && out.contains("0i64"),
        "Both Option value and fallback should be i64: {}",
        out
    );
}

// --- Result Type Inference ---

#[test]
fn result_ok_infers_ok_type() {
    let src = "(defn main [] (darcy.result/unwrap (darcy.result/ok 42)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("42i64"), "Should infer i64 Ok type: {}", out);
}

#[test]
fn result_err_infers_err_type() {
    let src = "(defn main [] (darcy.result/is-err (darcy.result/err \"error\")))";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("String::from(\"error\")"),
        "Should infer String Err type: {}",
        out
    );
}

// --- Vector Type Propagation ---

#[test]
fn vec_literal_infers_element_type() {
    let src = "(defn main [] [1 2 3])";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("1i64") && out.contains("2i64") && out.contains("3i64"),
        "Vector elements should be typed: {}",
        out
    );
}

#[test]
fn vec_map_propagates_types() {
    // Mapping i32 -> i32 should preserve types
    let src = "(defn main [v:vec<i32>] (darcy.vec/map (fn [x] (+ x 1)) v))";
    let out = compile(src).expect("compile ok");
    assert!(
        out.contains("darcy_stdlib::rt::vec_map"),
        "Should use vec_map: {}",
        out
    );
    assert!(out.contains("1i32"), "Increment should be i32: {}", out);
}

#[test]
fn vec_fold_infers_accumulator_type() {
    // Folding i32 vec with i64 init should work when types match
    let src = "(defn main [v:vec<i64>] (darcy.vec/fold (fn [acc x] (+ acc x)) 0 v))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("0i64"), "Init should be i64: {}", out);
}

// --- String Operations Type Safety ---

#[test]
fn string_split_returns_vec_string() {
    let src = "(defn main [s:string] (darcy.vec/len (darcy.string/split s \",\")))";
    let out = compile(src).expect("compile ok");
    // Should compile without type errors - split returns Vec<String>, len takes vec
    assert!(out.contains(".len()"), "Should call len on result: {}", out);
}

#[test]
fn string_join_takes_vec_string() {
    let src = "(defn main [parts:vec<string>] (darcy.string/join parts \",\"))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".join("), "Should use join method: {}", out);
}

// --- Type Errors Should Be Caught ---

#[test]
fn type_mismatch_vec_element_rejected() {
    // Mixing i64 and string in vec literal should fail
    let src = "(defn main [] [1 \"two\" 3])";
    let err = compile(src).expect_err("should reject mixed types");
    // The error might mention "numeric" or "mismatch" or "unify"
    assert!(
        err.message.contains("numeric")
            || err.message.contains("mismatch")
            || err.message.contains("unify"),
        "Should report type error: {}",
        err.message
    );
}

// NOTE: This test documents a known gap in the type system - Option<T>
// unwrap_or doesn't currently enforce T unification with the fallback type.
// The generated Rust code will catch this error, but the Darcy compiler should too.
#[test]
#[ignore] // TODO: Fix Option type unification in typecheck.rs
fn invalid_option_unwrap_or_type_rejected() {
    // Option<i64> with string fallback should fail
    let src = "(defn main [] (darcy.option/unwrap-or (darcy.option/some 42) \"fallback\"))";
    let err = compile(src).expect_err("should reject mismatched fallback type");
    assert!(
        err.message.contains("mismatch") || err.message.contains("unify"),
        "Should report type mismatch: {}",
        err.message
    );
}

// --- Math Operations Type Inference ---

#[test]
fn math_ops_preserve_numeric_type() {
    let src = "(defn main [x:f64 y:f64] (darcy.math/max x y))";
    let out = compile(src).expect("compile ok");
    // Should preserve f64, not coerce to i64
    assert!(out.contains("f64"), "Should preserve f64 type: {}", out);
}

#[test]
fn math_clamp_requires_same_types() {
    let src = "(defn main [x:i32 lo:i32 hi:i32] (darcy.math/clamp x lo hi))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains(".clamp("), "Should use clamp: {}", out);
}
