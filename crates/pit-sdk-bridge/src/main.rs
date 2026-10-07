use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process,
};

use pit_sdk_bridge::{emit_rust_shim, emit_typescript_shim, lower_sdk};
use portal_solutions_sdk::Sdk;

#[derive(Clone, Copy)]
enum Backend {
    Rust,
    TypeScript,
}

struct Config {
    root: PathBuf,
    sdk_files: Vec<PathBuf>,
    output: PathBuf,
    backend: Backend,
}

fn help() {
    eprintln!(
        "pit-sdk-gen — generate SDK↔PIT value conversion shims\n\
         \n\
         USAGE:\n\
         \tpit-sdk-gen --root <FILE> --backend <rust|ts> --out <FILE> [--sdk <FILE> ...]\n\
         \n\
         The root file is always included in the source catalog. Repeat --sdk for each\n\
         additional document referenced by SDK paths. PIT-only bindings remain generated\n\
         by pit-gen; this command is explicitly for paired SDK/PIT value shims.\n"
    );
}

fn parse_args() -> Config {
    let mut args = std::env::args().skip(1);
    let mut root = None;
    let mut sdk_files = Vec::new();
    let mut output = None;
    let mut backend = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                help();
                process::exit(0);
            }
            "--root" => root = Some(required_path(&mut args, "--root")),
            "--sdk" => sdk_files.push(required_path(&mut args, "--sdk")),
            "--out" => output = Some(required_path(&mut args, "--out")),
            "--backend" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("--backend requires rust or ts"));
                backend = Some(match value.as_str() {
                    "rust" => Backend::Rust,
                    "ts" => Backend::TypeScript,
                    _ => fail("supported shim backends are rust and ts"),
                });
            }
            unknown => fail(&format!("unknown argument {unknown}")),
        }
    }
    Config {
        root: root.unwrap_or_else(|| fail("--root is required")),
        sdk_files,
        output: output.unwrap_or_else(|| fail("--out is required")),
        backend: backend.unwrap_or_else(|| fail("--backend is required")),
    }
}

fn required_path(args: &mut impl Iterator<Item = String>, flag: &str) -> PathBuf {
    args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| fail(&format!("{flag} requires a path")))
}

fn fail<T>(message: &str) -> T {
    eprintln!("pit-sdk-gen: {message}");
    process::exit(2)
}

fn parse_document(path: &Path) -> Result<Sdk, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let (remaining, sdk) = Sdk::parse(&content)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    if !remaining.trim().is_empty() {
        return Err(format!(
            "unparsed trailing input in {}: {remaining:?}",
            path.display()
        ));
    }
    Ok(sdk)
}

fn write_output(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    if let Err(error) = fs::write(&temporary, content) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("cannot write {}: {error}", temporary.display()));
    }
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("cannot replace {}: {error}", path.display())
    })
}

fn run() -> Result<(), String> {
    let config = parse_args();
    let mut files = vec![config.root.clone()];
    files.extend(config.sdk_files);
    files.sort();
    files.dedup();

    let mut documents = BTreeMap::new();
    for path in &files {
        let key = path.to_string_lossy().into_owned();
        documents.insert(key, parse_document(path)?);
    }
    let root = config.root.to_string_lossy();
    let lowered = lower_sdk(&root, &documents).map_err(|error| error.to_string())?;
    let generated = match config.backend {
        Backend::Rust => emit_rust_shim(&lowered),
        Backend::TypeScript => emit_typescript_shim(&lowered),
    }
    .map_err(|error| error.to_string())?;
    write_output(&config.output, &generated)?;
    eprintln!("pit-sdk-gen: wrote {} paired shim", config.output.display());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("pit-sdk-gen: {error}");
        process::exit(1);
    }
}
