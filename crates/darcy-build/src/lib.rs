//! Build-time helpers for compiling Darcy sources into Rust during `cargo build`.
//!
//! Use from `build.rs`:
//! ```no_run
//! fn main() {
//!     darcy_build::Builder::new("darcy/main.dsl")
//!         .lib_path("darcy")
//!         .with_stdlib()
//!         .compile()
//!         .expect("darcy compile failed");
//! }
//! ```

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Builder {
    entry: PathBuf,
    lib_paths: Vec<PathBuf>,
    stdlib: Option<PathBuf>,
    out_file: Option<PathBuf>,
    emit_rerun: bool,
}

impl Builder {
    pub fn new(entry: impl Into<PathBuf>) -> Self {
        Self {
            entry: entry.into(),
            lib_paths: Vec::new(),
            stdlib: None,
            out_file: None,
            emit_rerun: true,
        }
    }

    pub fn lib_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.lib_paths.push(path.into());
        self
    }

    pub fn stdlib_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdlib = Some(path.into());
        self
    }

    pub fn with_stdlib(mut self) -> Self {
        self.stdlib = Some(stdlib_dir());
        self
    }

    pub fn out_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.out_file = Some(path.into());
        self
    }

    pub fn emit_rerun_if_changed(mut self, emit: bool) -> Self {
        self.emit_rerun = emit;
        self
    }

    pub fn compile(self) -> Result<PathBuf, String> {
        let entry = self.entry;
        let src = fs::read_to_string(&entry)
            .map_err(|e| format!("failed to read entry {}: {}", entry.display(), e))?;

        let mut lib_paths = self.lib_paths;
        if let Some(stdlib) = &self.stdlib {
            lib_paths.push(stdlib.clone());
        }

        if lib_paths.is_empty() {
            if let Some(parent) = entry.parent() {
                lib_paths.push(parent.to_path_buf());
            }
        }

        let file_label = entry.display().to_string();
        let output = dslc::compile_with_modules(&entry, &src, &lib_paths)
            .map_err(|e| dslc::render_diag(&file_label, &src, &e))?;

        let out_file = match self.out_file {
            Some(p) => p,
            None => {
                let out_dir = env::var("OUT_DIR")
                    .map_err(|e| format!("OUT_DIR not set; use Builder::out_file: {}", e))?;
                PathBuf::from(out_dir).join("darcy_gen.rs")
            }
        };

        if let Some(parent) = out_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
        }
        let rust = strip_inner_attrs(&output.rust);
        fs::write(&out_file, rust)
            .map_err(|e| format!("failed to write {}: {}", out_file.display(), e))?;

        for warn in &output.warnings {
            let rendered = dslc::render_diag_with_level(&file_label, &src, warn, "warning");
            for line in rendered.lines() {
                println!("cargo:warning={}", line);
            }
        }

        if self.emit_rerun {
            emit_rerun(&entry, &lib_paths)?;
        }

        Ok(out_file)
    }
}

fn strip_inner_attrs(src: &str) -> String {
    let mut out = String::new();
    for line in src.lines() {
        if line.trim_start().starts_with("#![") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn stdlib_dir() -> PathBuf {
    if let Ok(path) = env::var("DARCY_STDLIB") {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("crates/darcy-stdlib/darcy")
}

fn emit_rerun(entry: &Path, lib_paths: &[PathBuf]) -> Result<(), String> {
    println!("cargo:rerun-if-changed={}", entry.display());
    for path in lib_paths {
        for file in collect_dsl_files(path) {
            println!("cargo:rerun-if-changed={}", file.display());
        }
    }
    Ok(())
}

fn collect_dsl_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        let entries = match fs::read_dir(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if name.starts_with('.') || name == "target" {
                    continue;
                }
                pending.push(path);
            } else if path.extension().map(|e| e == "dsl").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    out
}
