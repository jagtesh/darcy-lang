use std::env;
use std::fs;

use dslc::{compile, render_diag};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        eprintln!("dslc (MVP)\n\nUsage:\n  dslc <input.dsl>\n\nOutputs Rust to stdout.\n");
        std::process::exit(2);
    }
    let file = args.remove(0);
    let src = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file, e);
            std::process::exit(1);
        }
    };

    match compile(&src) {
        Ok(rust) => {
            print!("{}", rust);
        }
        Err(d) => {
            eprintln!("{}", render_diag(&file, &src, &d));
            std::process::exit(1);
        }
    }
}
