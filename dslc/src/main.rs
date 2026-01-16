use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use dslc::{compile, render_diag};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        eprintln!(
            "dslc (MVP)\n\nUsage:\n  dslc <input.dsl>\n  dslc run <input.dsl>\n\nOutputs Rust to stdout, or compiles and runs with 'run'.\n"
        );
        std::process::exit(2);
    }
    let run_mode = args[0] == "run";
    if run_mode {
        args.remove(0);
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
            if run_mode {
                if let Err(e) = run_rust(&file, &rust) {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            } else {
                print!("{}", rust);
            }
        }
        Err(d) => {
            eprintln!("{}", render_diag(&file, &src, &d));
            std::process::exit(1);
        }
    }
}

fn run_rust(input: &str, rust_src: &str) -> Result<(), String> {
    let mut dir = env::temp_dir();
    dir.push("dslc_run");
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create temp dir: {}", e))?;
    let base = PathBuf::from(input);
    let base = base.file_stem().and_then(|s| s.to_str()).unwrap_or("dsl");
    let src_path = dir.join(format!("{}_gen.rs", base));
    let bin_path = dir.join(format!("{}_gen_bin", base));
    fs::write(&src_path, rust_src).map_err(|e| format!("cannot write rust source: {}", e))?;

    let status = Command::new("rustc")
        .arg(&src_path)
        .arg("-O")
        .arg("-o")
        .arg(&bin_path)
        .status()
        .map_err(|e| format!("failed to invoke rustc: {}", e))?;
    if !status.success() {
        return Err("rustc failed".to_string());
    }

    let status = Command::new(&bin_path)
        .status()
        .map_err(|e| format!("failed to run compiled binary: {}", e))?;
    if !status.success() {
        return Err("program exited with non-zero status".to_string());
    }
    Ok(())
}
