use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use dslc::{compile_with_modules, render_diag};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        eprintln!(
            "dslc (MVP)\n\nUsage:\n  dslc [--lib <dir>] <input.dsl>\n  dslc [--lib <dir>] run <input.dsl>\n\nOptions:\n  --lib, -L <dir>   Add a module search path (repeatable)\n\nOutputs Rust to stdout, or compiles and runs with 'run'.\n"
        );
        std::process::exit(2);
    }
    let mut run_mode = false;
    let mut lib_paths: Vec<PathBuf> = Vec::new();
    let mut files: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "run" => {
                run_mode = true;
                i += 1;
            }
            "--lib" | "-L" => {
                if i + 1 >= args.len() {
                    eprintln!("error: --lib requires a path");
                    std::process::exit(2);
                }
                lib_paths.push(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            _ => {
                files.push(args[i].clone());
                i += 1;
            }
        }
    }
    if files.len() != 1 {
        eprintln!("error: expected a single input file");
        std::process::exit(2);
    }
    let file = files.remove(0);
    let src = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file, e);
            std::process::exit(1);
        }
    };

    let input_path = PathBuf::from(&file);
    let input_dir = input_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut search_paths = lib_paths;
    if !search_paths.contains(&input_dir.to_path_buf()) {
        search_paths.push(input_dir.to_path_buf());
    }
    if let Ok(cwd) = env::current_dir() {
        if !search_paths.contains(&cwd) {
            search_paths.push(cwd);
        }
    }

    match compile_with_modules(&input_path, &src, &search_paths) {
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
