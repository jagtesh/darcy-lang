use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use dslc::{lex, parse_toplevel, typecheck_tops, Parser};

fn main() {
    let mut args = env::args().skip(1).peekable();
    match args.peek().map(|s| s.as_str()) {
        Some("compare") => {
            args.next();
            run_compare(args);
            return;
        }
        Some("update") => {
            args.next();
            run_update(args);
            return;
        }
        _ => {}
    }

    let mut iters = 1000usize;
    let mut save_path: Option<PathBuf> = None;
    let mut save_dir: Option<PathBuf> = None;
    let mut label: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iters" => {
                let v = args
                    .next()
                    .expect("--iters requires a value")
                    .parse::<usize>()
                    .expect("invalid --iters value");
                iters = v;
            }
            "--save" => {
                let v = args.next().expect("--save requires a path");
                save_path = Some(PathBuf::from(v));
            }
            "--save-dir" => {
                let v = args.next().expect("--save-dir requires a path");
                save_dir = Some(PathBuf::from(v));
            }
            "--label" => {
                let v = args.next().expect("--label requires a value");
                label = Some(v);
            }
            _ => {
                eprintln!("unknown argument: {}", arg);
                eprintln!(
                    "usage: bench_typecheck [--iters N] [--save path] [--save-dir dir] [--label name]"
                );
                eprintln!("       bench_typecheck compare --baseline path --candidate path");
                eprintln!("       bench_typecheck update --baseline path --candidate path");
                std::process::exit(2);
            }
        }
    }

    let src = r#"
(defrecord order (qty i32) (price f64))
(defn total [o] (* o.qty o.price))
(defn add1 [x] (+ x 1))
(defn scale [xs:vec<i32>] (* xs 2))
(defn apply [x] (+ x 1))
(defn main [] (print (total (order 1 2.5))))
"#;

    let toks = lex(src).expect("lex");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    let start = Instant::now();
    let mut acc = 0usize;
    for _ in 0..iters {
        let tc = typecheck_tops(&tops).expect("typecheck");
        acc += tc.typed_fns.len();
    }
    let elapsed = start.elapsed();
    let elapsed_ns = elapsed.as_nanos();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let per_iter_ns = elapsed_ns as f64 / iters as f64;
    let per_iter_us = per_iter_ns / 1000.0;

    println!(
        "typecheck: {:.3}ms total, {:.3}us/iter (iters={}, acc={})",
        elapsed_ms, per_iter_us, iters, acc
    );

    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let label_json = label
        .as_ref()
        .map(|s| json_escape(s))
        .unwrap_or_else(|| "".to_string());
    let json = format!(
        "{{\"iters\":{},\"elapsed_ns\":{},\"elapsed_ms\":{:.3},\"per_iter_ns\":{:.3},\"per_iter_us\":{:.3},\"label\":\"{}\",\"timestamp_ns\":{}}}\n",
        iters, elapsed_ns, elapsed_ms, per_iter_ns, per_iter_us, label_json, timestamp_ns
    );

    if let Some(path) = save_path {
        write_json(&path, &json);
        println!("saved results to {}", path.display());
    }
    if let Some(dir) = save_dir {
        let filename = format_history_filename(label.as_deref(), timestamp_ns);
        let path = dir.join(filename);
        write_json(&path, &json);
        println!("saved results to {}", path.display());
    }
}

fn run_compare(mut args: impl Iterator<Item = String>) {
    let mut baseline: Option<PathBuf> = None;
    let mut candidate: Option<PathBuf> = None;
    let mut max_regression_pct: Option<f64> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--baseline" => {
                baseline = Some(PathBuf::from(
                    args.next().expect("--baseline requires a path"),
                ));
            }
            "--candidate" => {
                candidate = Some(PathBuf::from(
                    args.next().expect("--candidate requires a path"),
                ));
            }
            "--max-regression-pct" => {
                let v = args
                    .next()
                    .expect("--max-regression-pct requires a value")
                    .parse::<f64>()
                    .expect("invalid --max-regression-pct value");
                max_regression_pct = Some(v);
            }
            _ => {
                eprintln!("unknown argument: {}", arg);
                eprintln!(
                    "usage: bench_typecheck compare --baseline path --candidate path [--max-regression-pct N]"
                );
                std::process::exit(2);
            }
        }
    }
    let baseline = baseline.expect("--baseline is required");
    let candidate = candidate.expect("--candidate is required");

    let base = read_metric(&baseline, "per_iter_us").expect("read baseline metric");
    let cand = read_metric(&candidate, "per_iter_us").expect("read candidate metric");
    let delta = cand - base;
    let pct = if base.abs() < f64::EPSILON {
        0.0
    } else {
        (delta / base) * 100.0
    };

    println!(
        "compare: baseline={:.3}us/iter, candidate={:.3}us/iter, delta={:+.3}us ({:+.2}%)",
        base, cand, delta, pct
    );

    if let Some(max_pct) = max_regression_pct {
        if delta > 0.0 && pct > max_pct {
            eprintln!("regression: +{:.2}% exceeds max {:.2}%", pct, max_pct);
            std::process::exit(1);
        }
    }
}

fn run_update(mut args: impl Iterator<Item = String>) {
    let mut baseline: Option<PathBuf> = None;
    let mut candidate: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--baseline" => {
                baseline = Some(PathBuf::from(
                    args.next().expect("--baseline requires a path"),
                ));
            }
            "--candidate" => {
                candidate = Some(PathBuf::from(
                    args.next().expect("--candidate requires a path"),
                ));
            }
            _ => {
                eprintln!("unknown argument: {}", arg);
                eprintln!("usage: bench_typecheck update --baseline path --candidate path");
                std::process::exit(2);
            }
        }
    }
    let baseline = baseline.expect("--baseline is required");
    let candidate = candidate.expect("--candidate is required");
    if let Some(parent) = baseline.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("create baseline directory");
        }
    }
    fs::copy(&candidate, &baseline).expect("update baseline");
    println!(
        "updated baseline: {} <- {}",
        baseline.display(),
        candidate.display()
    );
}

fn format_history_filename(label: Option<&str>, timestamp_ns: u128) -> String {
    match label {
        Some(l) if !l.is_empty() => format!("typecheck_{}_{}.json", l, timestamp_ns),
        _ => format!("typecheck_{}.json", timestamp_ns),
    }
}

fn write_json(path: &Path, json: &str) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("create save directory");
        }
    }
    fs::write(path, json).expect("write bench results");
}

fn read_metric(path: &Path, key: &str) -> Result<f64, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let needle = format!("\"{}\":", key);
    let start = raw
        .find(&needle)
        .ok_or_else(|| format!("missing '{}' in {}", key, path.display()))?;
    let mut idx = start + needle.len();
    let bytes = raw.as_bytes();
    while idx < bytes.len()
        && (bytes[idx] == b' ' || bytes[idx] == b'\n' || bytes[idx] == b'\r' || bytes[idx] == b'\t')
    {
        idx += 1;
    }
    let mut end = idx;
    while end < bytes.len() {
        let c = bytes[end] as char;
        if c.is_ascii_digit() || c == '.' || c == '-' {
            end += 1;
        } else {
            break;
        }
    }
    let val = raw[idx..end]
        .parse::<f64>()
        .map_err(|e| format!("parse {} in {}: {}", key, path.display(), e))?;
    Ok(val)
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\"', "\\\"")
}
