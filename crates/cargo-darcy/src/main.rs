use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    match cmd {
        Some(ref name) if name == "init" => {
            let target = args.next().unwrap_or_else(|| ".".to_string());
            if let Err(err) = cmd_init(&target) {
                eprintln!("cargo-darcy init failed: {}", err);
                std::process::exit(1);
            }
        }
        Some(ref name) if name == "run" => {
            if let Err(err) = cmd_run() {
                eprintln!("cargo-darcy run failed: {}", err);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("cargo-darcy\n\nUsage:\n  cargo darcy init <path>\n  cargo darcy run\n");
            std::process::exit(2);
        }
    }
}

fn cmd_init(target: &str) -> Result<(), String> {
    let target_dir = PathBuf::from(target);
    let cargo_toml = target_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        run_command(
            Command::new("cargo")
                .arg("init")
                .arg(&target_dir)
                .arg("--bin"),
        )?;
    }

    scaffold_darcy(&target_dir)?;
    Ok(())
}

fn cmd_run() -> Result<(), String> {
    run_command(Command::new("cargo").arg("run"))
}

fn scaffold_darcy(target_dir: &Path) -> Result<(), String> {
    let darcy_dir = target_dir.join("darcy");
    fs::create_dir_all(&darcy_dir)
        .map_err(|e| format!("failed to create {}: {}", darcy_dir.display(), e))?;

    let main_dsl = darcy_dir.join("main.dsl");
    if !main_dsl.exists() {
        let src = "(defn answer [] 42)\n";
        fs::write(&main_dsl, src)
            .map_err(|e| format!("failed to write {}: {}", main_dsl.display(), e))?;
    }

    let build_rs = target_dir.join("build.rs");
    if !build_rs.exists() {
        let build_src = r#"use std::path::PathBuf;

fn main() {
    if std::env::var("CARGO_FEATURE_DARCY_COMPILED").is_err() {
        return;
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let entry = manifest_dir.join("darcy/main.dsl");
    let lib_dir = manifest_dir.join("darcy");
    let stdlib = std::env::var("DARCY_STDLIB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| darcy_stdlib::stdlib_dir());

    darcy_build::Builder::new(entry)
        .lib_path(lib_dir)
        .stdlib_path(stdlib)
        .compile()
        .expect("darcy compile failed");
}
"#;
        fs::write(&build_rs, build_src)
            .map_err(|e| format!("failed to write {}: {}", build_rs.display(), e))?;
    }

    let src_lib = target_dir.join("src/lib.rs");
    if !src_lib.exists() {
        write_lib_rs(&src_lib)?;
    }

    let src_main = target_dir.join("src/main.rs");
    if !src_main.exists() {
        write_main_rs(&src_main, target_dir)?;
    } else if let Ok(existing) = fs::read_to_string(&src_main) {
        if existing.contains("Hello, world!") {
            write_main_rs(&src_main, target_dir)?;
        }
    }

    update_cargo_toml(&target_dir.join("Cargo.toml"))?;
    Ok(())
}

fn write_lib_rs(path: &Path) -> Result<(), String> {
    let src = r#"use std::path::PathBuf;

pub fn darcy_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("darcy")
}

#[cfg(feature = "darcy-compiled")]
pub mod darcy_gen {
    #![allow(dead_code)]
    #![allow(unused_parens)]
    #![allow(clippy::redundant_pattern)]
    #![allow(non_shorthand_field_patterns)]
    #![allow(unused_braces)]
    include!(concat!(env!("OUT_DIR"), "/darcy_gen.rs"));
}

#[cfg(feature = "darcy-compiled")]
pub use darcy_gen::*;
"#;
    fs::write(path, src).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn write_main_rs(path: &Path, target_dir: &Path) -> Result<(), String> {
    let name =
        crate_name_from_toml(&target_dir.join("Cargo.toml")).unwrap_or_else(|| "crate".to_string());
    let src = format!(
        r#"fn main() {{
    let answer = {name}::answer();
    println!("answer = {{}}", answer);
}}
"#,
        name = name
    );
    fs::write(path, src).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn update_cargo_toml(path: &Path) -> Result<(), String> {
    let mut toml = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;

    ensure_feature_section(&mut toml);

    if use_sdk_deps() {
        replace_dependency(
            &mut toml,
            "dependencies",
            "darcy-stdlib",
            &darcy_stdlib_spec(),
        );
        replace_dependency(
            &mut toml,
            "build-dependencies",
            "darcy-build",
            &darcy_build_spec(),
        );
        replace_dependency(
            &mut toml,
            "build-dependencies",
            "darcy-stdlib",
            &darcy_stdlib_spec(),
        );
    }

    if !toml.contains("darcy-build") || !toml.contains("darcy-stdlib") {
        let dep_line = format!(
            "darcy-build = {}\ndarcy-stdlib = {}\n",
            darcy_build_spec(),
            darcy_stdlib_spec()
        );
        if let Some(idx) = toml.find("[build-dependencies]") {
            let rest = &toml[idx + "[build-dependencies]".len()..];
            let next_section = rest
                .find("\n[")
                .map(|off| idx + "[build-dependencies]".len() + off);
            let insert_at = next_section.unwrap_or_else(|| toml.len());
            toml.insert_str(insert_at, &format!("\n{}", dep_line));
        } else {
            if !toml.ends_with('\n') {
                toml.push('\n');
            }
            toml.push_str("\n[build-dependencies]\n");
            toml.push_str(&dep_line);
        }
    }

    if !section_contains(&toml, "dependencies", "darcy-stdlib") {
        let dep_line = format!("darcy-stdlib = {}\n", darcy_stdlib_spec());
        if let Some(idx) = toml.find("[dependencies]") {
            let rest = &toml[idx + "[dependencies]".len()..];
            let next_section = rest
                .find("\n[")
                .map(|off| idx + "[dependencies]".len() + off);
            let insert_at = next_section.unwrap_or_else(|| toml.len());
            toml.insert_str(insert_at, &format!("\n{}", dep_line));
        } else {
            if !toml.ends_with('\n') {
                toml.push('\n');
            }
            toml.push_str("\n[dependencies]\n");
            toml.push_str(&dep_line);
        }
    }

    fs::write(path, toml).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn use_sdk_deps() -> bool {
    env::var("DARCY_SDK").is_ok()
}

fn darcy_build_spec() -> String {
    if let Ok(root) = env::var("DARCY_SDK") {
        let path = PathBuf::from(root).join("crates/darcy-build");
        format!("{{ path = \"{}\" }}", path.display())
    } else {
        "\"0.1.0\"".to_string()
    }
}

fn darcy_stdlib_spec() -> String {
    if let Ok(root) = env::var("DARCY_SDK") {
        let path = PathBuf::from(root).join("crates/darcy-stdlib");
        format!("{{ path = \"{}\" }}", path.display())
    } else {
        "\"0.1.0\"".to_string()
    }
}

fn ensure_feature_section(toml: &mut String) {
    if toml.contains("darcy-compiled") {
        return;
    }
    if let Some(idx) = toml.find("[features]") {
        let rest = &toml[idx + "[features]".len()..];
        let next_section = rest.find("\n[").map(|off| idx + "[features]".len() + off);
        let insert_at = next_section.unwrap_or_else(|| toml.len());
        toml.insert_str(
            insert_at,
            "\n\
darcy-compiled = []\n\
default = [\"darcy-compiled\"]\n",
        );
    } else {
        if !toml.ends_with('\n') {
            toml.push('\n');
        }
        toml.push_str("\n[features]\n");
        toml.push_str("darcy-compiled = []\n");
        toml.push_str("default = [\"darcy-compiled\"]\n");
    }
}

fn replace_dependency(toml: &mut String, section: &str, key: &str, spec: &str) {
    let header = format!("[{}]", section);
    let start = match toml.find(&header) {
        Some(idx) => idx + header.len(),
        None => {
            if !toml.ends_with('\n') {
                toml.push('\n');
            }
            toml.push_str(&format!("\n{}\n", header));
            toml.push_str(&format!("{} = {}\n", key, spec));
            return;
        }
    };
    let rest = &toml[start..];
    let end = rest
        .find("\n[")
        .map(|off| start + off)
        .unwrap_or_else(|| toml.len());
    let section_text = &toml[start..end];
    let mut replaced = false;
    let mut out = String::new();
    for line in section_text.lines() {
        let line_trim = line.trim();
        if line_trim.starts_with('#') || line_trim.is_empty() {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if line_trim.starts_with(key) {
            out.push_str(&format!("{} = {}", key, spec));
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        out.push_str(&format!("{} = {}", key, spec));
        out.push('\n');
    }
    toml.replace_range(start..end, &out);
}

fn section_contains(toml: &str, section: &str, key: &str) -> bool {
    let header = format!("[{}]", section);
    let start = match toml.find(&header) {
        Some(idx) => idx + header.len(),
        None => return false,
    };
    let rest = &toml[start..];
    let end = rest
        .find("\n[")
        .map(|off| start + off)
        .unwrap_or_else(|| toml.len());
    toml[start..end].lines().any(|line| {
        let line = line.trim();
        !line.starts_with('#') && line.starts_with(key)
    })
}

fn crate_name_from_toml(path: &Path) -> Option<String> {
    let toml = fs::read_to_string(path).ok()?;
    let mut in_package = false;
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package && line.starts_with("name") {
            let mut parts = line.splitn(2, '=');
            let _ = parts.next()?;
            let value = parts.next()?.trim();
            let value = value.trim_matches('"');
            if !value.is_empty() {
                return Some(value.replace('-', "_"));
            }
        }
    }
    None
}

fn run_command(cmd: &mut Command) -> Result<(), String> {
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run command: {}", e))?;
    if !status.success() {
        return Err(format!("command failed with status {}", status));
    }
    Ok(())
}
