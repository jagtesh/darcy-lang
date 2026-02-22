use dslc::compile;

#[test]
fn borrows_params_for_read_only_use() {
    let src = "(defn total-len [v:vec<i32>] (do (darcy.vec/len v) (darcy.vec/len v)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("fn total_len(v: &Arc<Vec<i32>>)"), "{}", out);
}

#[test]
fn auto_clones_on_reuse_after_move() {
    let src = "(defn double-map [v:vec<i32>] (do (darcy.vec/map (fn [x:i32] x) v) (darcy.vec/map (fn [x:i32] x) v)))";
    let out = compile(src).expect("compile ok");
    assert!(out.contains("(v).clone()"), "{}", out);
}
