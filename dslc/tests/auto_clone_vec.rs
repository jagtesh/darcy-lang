use dslc::compile;

fn count_clones(out: &str, name: &str) -> usize {
    let mut count = 0usize;
    let pat1 = format!("({}).clone()", name);
    let pat2 = format!("{}.clone()", name);
    count += out.matches(&pat1).count();
    count += out.matches(&pat2).count();
    count
}

#[test]
fn clones_on_multiple_calls() {
    let src = r#"
(defn ema [prices:vec<f64> period:i64]
  (darcy.vec/len prices))

(defn run-strategy [prices:vec<f64> fast-period:i64 slow-period:i64]
  (let [fast (ema prices fast-period)
        slow (ema prices slow-period)]
    (+ fast slow)))
"#;
    let out = compile(src).expect("compile ok");
    assert!(count_clones(&out, "prices") >= 1, "{}", out);
}

#[test]
fn clones_on_use_after_call() {
    let src = r#"
(defn run-strategy [prices:vec<f64>] (darcy.vec/len prices))

(defn main []
  (let [market-data (darcy.vec/repeat 0.0 10)]
    (let [preview (darcy.vec/take market-data 5)]
      (darcy.io/dbg preview))
    (run-strategy market-data)))
"#;
    let out = compile(src).expect("compile ok");
    assert!(count_clones(&out, "market_data") >= 1, "{}", out);
}

#[test]
fn clones_in_loop_body() {
    let src = r#"
(defn ema [prices:vec<f64>]
  (let [start-val (darcy.vec/get prices 0)]
    (for i (range 0 2)
      (let [p (darcy.vec/get prices i)]
        p))
    start-val))
"#;
    let out = compile(src).expect("compile ok");
    assert!(count_clones(&out, "prices") >= 1, "{}", out);
}
