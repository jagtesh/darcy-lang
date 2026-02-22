use dslc::compile;

#[test]
fn test_case_redundant_wildcard() {
    let src = "
(defenum e-type (a) (b))

(defn test-redundant [e:e-type]
  (case e
    (a 1)
    (b 2)
    (_ 3)))
";
    // Should succeed
    let out = compile(src).expect("compile ok");

    // Check that generated code DOES NOT contain the wildcard arm
    // Once implemented, this should pass. Currently it will likely fail.
    assert!(
        !out.contains("_ =>"),
        "Generated code contained fallback wildcard: {}",
        out
    );
}

#[test]
fn test_case_missing_arm() {
    let src = "
(defenum e-type (a) (b))

(defn test-missing [e:e-type]
  (case e
    (a 1)))
";
    // Should fail
    let err = compile(src).expect_err("expected error");
    assert!(
        err.message.contains("non-exhaustive case"),
        "{}",
        err.message
    );
}
