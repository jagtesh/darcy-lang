use dslc::compile;

#[test]
fn broadcasts_vector_scalar() {
    let src = "(defn scale [] (* [1 2 3] 2))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("iter().map(|__x| (*__x) * 2i64)"), "{}", out);
}

#[test]
fn broadcasts_struct_field_access() {
    let src = "(defrecord order [qty:u32]) (defn qtys [os:vec<order>] os.qty)";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("iter().map(|__x| __x.qty.clone())"), "{}", out);
}

#[test]
fn rejects_vector_scalar_mismatch() {
    let src = "(defn scale [] (* [1 2 3] 2.0))";
    let err = compile(src).expect_err("expected type error");
    assert!(
        err.message.contains("numeric operator types"),
        "{}",
        err.message
    );
}

#[test]
fn vec_get_lowers() {
    let src = "(defn main [v:vec<i32>] (darcy.vec/get v 0))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_get"), "{}", out);
}

#[test]
fn vec_set_lowers() {
    let src = "(defn main [v:vec<i32>] (darcy.vec/set v 0 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_set"), "{}", out);
}

#[test]
fn vec_new_lowers() {
    let src = "(defn main [] (let [v:vec<i32> (darcy.vec/new)] v))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_new"), "{}", out);
}

#[test]
fn vec_repeat_lowers() {
    let src = "(defn main [] (darcy.vec/repeat 1.5 4))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_repeat"), "{}", out);
}

#[test]
fn vec_push_lowers() {
    let src = "(defn main [v:vec<i32>] (darcy.vec/push v 1))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_push"), "{}", out);
}

#[test]
fn vec_map_lowers() {
    let src = "(defn main [v:vec<i32>] (darcy.vec/map (fn [x] (+ x 1)) v))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_map"), "{}", out);
}

#[test]
fn vec_map2_lowers() {
    let src = "(defn main [a:vec<i32> b:vec<i32>] (darcy.vec/map2 (fn [x y] (+ x y)) a b))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_map2"), "{}", out);
}

#[test]
fn vec_fold_lowers() {
    let src = "(defn main [v:vec<i32>] (darcy.vec/fold (fn [acc x] (+ acc x)) 0 v))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_fold"), "{}", out);
}

#[test]
fn vec_take_lowers() {
    let src = "(defn main [v:vec<i32>] (darcy.vec/take v 3))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_take"), "{}", out);
}

#[test]
fn vec_range_lowers() {
    let src = "(defn main [n:i64] (darcy.vec/range n))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("darcy_stdlib::rt::vec_range"), "{}", out);
}
