use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use ide::{AnalysisHost, AssistResolveStrategy, DiagnosticsConfig};
use load_cargo::{load_workspace_at, LoadCargoConfig, ProcMacroServerChoice};
use paths::AbsPathBuf;
use project_model::{CargoConfig, RustLibSource};
use vfs::VfsPath;

use crate::ast::Ty;
use crate::diag::{Diag, DslResult};
use crate::typecheck::TypecheckedProgram;
use crate::typed::GenericBound;

#[derive(Debug, Default)]
pub struct FeedbackHints {
    pub bounds: BTreeMap<u32, Vec<GenericBound>>,
}

pub fn collect_feedback_hints(rust_src: &str) -> DslResult<FeedbackHints> {
    let stdlib = stdlib_path()?;
    let mut dir = env::temp_dir();
    dir.push(format!("dslc_feedback_{}", std::process::id()));
    fs::create_dir_all(&dir)
        .map_err(|e| Diag::new(format!("feedback: cannot create temp dir: {}", e)))?;
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir)
        .map_err(|e| Diag::new(format!("feedback: cannot create src dir: {}", e)))?;
    let main_path = src_dir.join("main.rs");
    fs::write(&main_path, rust_src)
        .map_err(|e| Diag::new(format!("feedback: cannot write main.rs: {}", e)))?;
    let cargo_toml = dir.join("Cargo.toml");
    let cargo_contents = format!(
        "[package]\nname = \"dslc_feedback\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ndarcy-stdlib = {{ path = \"{}\" }}\n",
        stdlib.display()
    );
    fs::write(&cargo_toml, cargo_contents)
        .map_err(|e| Diag::new(format!("feedback: cannot write Cargo.toml: {}", e)))?;

    let rustc_dir = dir.clone();
    let ra_dir = dir.clone();
    let ra_main = main_path.clone();

    let rustc_handle = thread::spawn(move || collect_rustc_feedback(&rustc_dir));
    let ra_handle = thread::spawn(move || collect_ra_feedback(&ra_dir, &ra_main));

    let rustc_hints = join_handle(rustc_handle, "rustc")?;
    let ra_hints = join_handle(ra_handle, "rust-analyzer")?;

    Ok(merge_feedback_hints(rustc_hints, ra_hints))
}

fn join_handle(
    handle: thread::JoinHandle<DslResult<FeedbackHints>>,
    label: &str,
) -> DslResult<FeedbackHints> {
    match handle.join() {
        Ok(res) => res,
        Err(_) => Err(Diag::new(format!("feedback: {} thread panicked", label))),
    }
}

fn collect_rustc_feedback(dir: &Path) -> DslResult<FeedbackHints> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--message-format=json")
        .current_dir(dir)
        .output()
        .map_err(|e| Diag::new(format!("feedback: failed to invoke cargo: {}", e)))?;

    let mut hints = FeedbackHints::default();
    parse_diagnostics(&output.stdout, &mut hints);
    parse_diagnostics(&output.stderr, &mut hints);
    Ok(hints)
}

fn collect_ra_feedback(dir: &Path, main_path: &Path) -> DslResult<FeedbackHints> {
    let cargo_config = CargoConfig {
        sysroot: Some(RustLibSource::Discover),
        all_targets: true,
        ..Default::default()
    };
    let load_cargo_config = LoadCargoConfig {
        load_out_dirs_from_check: true,
        with_proc_macro_server: ProcMacroServerChoice::Sysroot,
        prefill_caches: true,
        proc_macro_processes: 1,
    };
    let (db, vfs, _proc_macro) = load_workspace_at(dir, &cargo_config, &load_cargo_config, &|_| {})
        .map_err(|e| Diag::new(format!("feedback: rust-analyzer load failed: {}", e)))?;

    let host = AnalysisHost::with_database(db);
    let analysis = host.analysis();

    let abs_main = AbsPathBuf::assert_utf8(main_path.to_path_buf());
    let vfs_path = VfsPath::from(abs_main);
    let (file_id, _excluded) = vfs
        .file_id(&vfs_path)
        .ok_or_else(|| Diag::new("feedback: rust-analyzer cannot find main.rs in VFS"))?;

    let diags = analysis
        .full_diagnostics(
            &DiagnosticsConfig::test_sample(),
            AssistResolveStrategy::None,
            file_id,
        )
        .map_err(|_| Diag::new("feedback: rust-analyzer diagnostics cancelled"))?;

    let mut hints = FeedbackHints::default();
    for diag in diags {
        collect_bounds_from_text(&diag.message, &mut hints.bounds);
    }
    Ok(hints)
}

fn merge_feedback_hints(mut rustc: FeedbackHints, ra: FeedbackHints) -> FeedbackHints {
    for (id, bounds) in ra.bounds {
        let entry = rustc.bounds.entry(id).or_insert_with(Vec::new);
        for bound in bounds {
            if !entry.iter().any(|b| *b == bound) {
                entry.push(bound);
            }
        }
    }
    rustc
}

fn parse_diagnostics(buf: &[u8], hints: &mut FeedbackHints) {
    for line in String::from_utf8_lossy(buf).lines() {
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if value.get("reason").and_then(|v| v.as_str()) != Some("compiler-message") {
            continue;
        }
        if let Some(message) = value.get("message") {
            collect_message_bounds(message, &mut hints.bounds);
        }
    }
}

pub fn apply_feedback_hints(program: &mut TypecheckedProgram, hints: &FeedbackHints) {
    if hints.bounds.is_empty() {
        return;
    }
    for f in &mut program.typed_fns {
        let mut ids = BTreeSet::new();
        for ty in f.param_tys.values() {
            collect_generic_ids(ty, &mut ids);
        }
        collect_generic_ids(&f.body.ty, &mut ids);
        for id in ids {
            if let Some(bounds) = hints.bounds.get(&id) {
                for bound in bounds {
                    add_bound(&mut f.generic_bounds, id, bound.clone());
                }
            }
        }
    }
}

fn collect_message_bounds(message: &serde_json::Value, out: &mut BTreeMap<u32, Vec<GenericBound>>) {
    if let Some(text) = message.get("message").and_then(|v| v.as_str()) {
        collect_bounds_from_text(text, out);
    }
    if let Some(children) = message.get("children").and_then(|v| v.as_array()) {
        for child in children {
            collect_message_bounds(child, out);
        }
    }
}

fn collect_bounds_from_text(text: &str, out: &mut BTreeMap<u32, Vec<GenericBound>>) {
    if let Some((id, bound)) = parse_trait_bound(text) {
        add_bound(out, id, bound);
    }
    if let Some((id, bound)) = parse_restricting_type_param(text) {
        add_bound(out, id, bound);
    }
    if let Some((id, bound)) = parse_missing_impl(text) {
        add_bound(out, id, bound);
    }
}

fn parse_trait_bound(message: &str) -> Option<(u32, GenericBound)> {
    let needle = "trait bound `";
    let start = message.find(needle)?;
    let rest = &message[start + needle.len()..];
    let end = rest.find('`')?;
    let bound = &rest[..end];
    parse_bound_string(bound)
}

fn parse_bound_string(bound: &str) -> Option<(u32, GenericBound)> {
    let mut parts = bound.splitn(2, ':');
    let left = parts.next()?.trim();
    let right = parts.next()?.trim();
    let id = parse_generic_id(left)?;
    let bound = match right {
        "Copy" => GenericBound::Copy,
        "Clone" => GenericBound::Clone,
        "Debug" => GenericBound::Debug,
        "PartialEq" => GenericBound::PartialEq,
        "PartialOrd" => GenericBound::PartialOrd,
        "__DarcyLen" => GenericBound::Len,
        "darcy_stdlib::rt::FromInt" => GenericBound::FromInt,
        _ if right.starts_with("std::ops::Add") => GenericBound::Add,
        _ if right.starts_with("std::ops::Sub") => GenericBound::Sub,
        _ if right.starts_with("std::ops::Mul") => GenericBound::Mul,
        _ if right.starts_with("std::ops::Div") => GenericBound::Div,
        _ => return None,
    };
    Some((id, bound))
}

fn parse_restricting_type_param(message: &str) -> Option<(u32, GenericBound)> {
    let needle = "consider restricting type parameter `";
    let start = message.find(needle)?;
    let rest = &message[start + needle.len()..];
    let end = rest.find('`')?;
    let type_param = &rest[..end];
    let after = &rest[end + 1..];
    let trait_needle = "trait `";
    let trait_start = after.find(trait_needle)?;
    let trait_rest = &after[trait_start + trait_needle.len()..];
    let trait_end = trait_rest.find('`')?;
    let trait_name = &trait_rest[..trait_end];
    let id = parse_generic_id(type_param)?;
    let bound = match trait_name {
        "Debug" | "std::fmt::Debug" => GenericBound::Debug,
        _ => return None,
    };
    Some((id, bound))
}

fn parse_missing_impl(message: &str) -> Option<(u32, GenericBound)> {
    if !message.contains("doesn't implement") {
        return None;
    }
    let first = message.find('`')?;
    let rest = &message[first + 1..];
    let second = rest.find('`')?;
    let type_param = &rest[..second];
    let after = &rest[second + 1..];
    let third = after.find('`')?;
    let trait_rest = &after[third + 1..];
    let fourth = trait_rest.find('`')?;
    let trait_name = &trait_rest[..fourth];
    let id = parse_generic_id(type_param)?;
    let bound = match trait_name {
        "Debug" | "std::fmt::Debug" => GenericBound::Debug,
        _ => return None,
    };
    Some((id, bound))
}

fn parse_generic_id(name: &str) -> Option<u32> {
    let mut chars = name.chars();
    if chars.next()? != 'T' {
        return None;
    }
    let digits: String = chars.take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

fn add_bound(bounds: &mut BTreeMap<u32, Vec<GenericBound>>, id: u32, bound: GenericBound) {
    let entry = bounds.entry(id).or_insert_with(Vec::new);
    if !entry.iter().any(|b| *b == bound) {
        entry.push(bound);
    }
}

fn collect_generic_ids(ty: &Ty, out: &mut BTreeSet<u32>) {
    match ty {
        Ty::Generic(id) => {
            out.insert(*id);
        }
        Ty::Vec(inner) | Ty::Set(inner) | Ty::Option(inner) => {
            collect_generic_ids(inner, out);
        }
        Ty::Result(ok, err) => {
            collect_generic_ids(ok, out);
            collect_generic_ids(err, out);
        }
        Ty::Map(_, k, v) => {
            collect_generic_ids(k, out);
            collect_generic_ids(v, out);
        }
        Ty::Union(items) => {
            for item in items {
                collect_generic_ids(item, out);
            }
        }
        Ty::Named(_) | Ty::Unknown => {}
    }
}

fn stdlib_path() -> DslResult<PathBuf> {
    let cwd =
        env::current_dir().map_err(|e| Diag::new(format!("feedback: cannot get cwd: {}", e)))?;
    let candidate = cwd.join("crates/darcy-stdlib");
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(Diag::new(
        "feedback: darcy-stdlib not found; run from workspace root",
    ))
}
