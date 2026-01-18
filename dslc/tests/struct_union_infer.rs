use dslc::compile;

#[test]
fn infers_struct_field_types() {
    let src = "(defstruct point (x) (y)) (defn get-x [p] p.x) (defn main [] (get-x (point 1 2)))";
    let out = compile(src).expect("expected inference to succeed");
    assert!(out.contains("struct Point"));
}

#[test]
fn infers_union_field_types_from_match() {
    let src = "(defunion opt (some (v)) (none)) (defn get [o] (match o (some (v v) v) (none 0))) (defn main [] (get (some 1)))";
    let out = compile(src).expect("expected inference to succeed");
    assert!(out.contains("enum Opt"));
}

#[test]
fn infers_param_types_from_call_sites() {
    let src = "(defn id [x] x) (defn main [] (id 1))";
    let out = compile(src).expect("expected inference to succeed");
    assert!(out.contains("fn id"));
}
