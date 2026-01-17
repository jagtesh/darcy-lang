use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use dslc::{lex, parse_toplevel, typecheck_tops, Parser};

fn main() {
    let mut iters = 1000usize;
    let mut save_path: Option<PathBuf> = None;

    let mut args = env::args().skip(1);
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
            _ => {
                eprintln!("unknown argument: {}", arg);
                eprintln!("usage: bench_typecheck [--iters N] [--save path]");
                std::process::exit(2);
            }
        }
    }

    let src = r#"
(defstruct order (qty i32) (price f64))
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

    if let Some(path) = save_path {
        let json = format!(
            "{{\"iters\":{},\"elapsed_ns\":{},\"elapsed_ms\":{:.3},\"per_iter_ns\":{:.3},\"per_iter_us\":{:.3}}}\n",
            iters, elapsed_ns, elapsed_ms, per_iter_ns, per_iter_us
        );
        fs::write(&path, json).expect("write bench results");
        println!("saved results to {}", path.display());
    }
}
