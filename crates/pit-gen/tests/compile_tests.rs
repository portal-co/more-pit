//! Compile tests for pit-gen.
//!
//! Each test generates bindings for all `.pit` files in the repository
//! (relative path `../../pit/`) into a temporary directory, then invokes
//! the real language compiler to type-check the output.
//!
//! Tests are skipped automatically when the required compiler is not on `PATH`.
//!
//! Run with:
//! ```text
//! cargo test --package pit-gen -- --nocapture
//! ```

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use pit_core::{parse_interface, Arg, Interface, ResTy};
use pit_gen::{direct_deps, output_filename, rewrite_value, Backend};
use pit_lang_generic::{
    C, Go, Haskell, Haxe, Java, Opts, Scala, Swift, Syntax, TypeScript, TypeScriptAsync,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Return the absolute path to the `pit/` directory in this repository.
fn pit_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../pit")
        .canonicalize()
        .expect("pit/ directory should exist")
}

/// Collect all *.pit files under `dir` (recursive).
fn collect_pit_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_recursive(dir, &mut out);
    out.sort();
    out
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("pit") {
            out.push(path);
        }
    }
}

struct PitFile {
    iface: Interface,
}

/// Parse all .pit files, return a RID → PitFile map.  Files that fail to
/// parse are skipped with a warning (e.g. `age.pit` has complex attributes).
fn parse_all(paths: &[PathBuf]) -> BTreeMap<[u8; 32], PitFile> {
    let mut map = BTreeMap::new();
    for path in paths {
        let src = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let iface = match parse_interface(&src) {
            Ok((_, iface)) => iface,
            Err(e) => {
                eprintln!("compile_tests: skipping {} ({})", path.display(), e);
                continue;
            }
        };
        map.insert(iface.rid(), PitFile { iface });
    }
    map
}

/// Generate all files for a syntax backend into `dir`.
fn generate_syntax<S: Syntax>(
    backend: Backend,
    all: &BTreeMap<[u8; 32], PitFile>,
    dir: &Path,
    setup: impl Fn(&mut Opts<S>),
) {
    fs::create_dir_all(dir).unwrap();
    for (rid, pf) in all {
        let hex = hex::encode(rid);
        let fname = output_filename(backend, &hex);
        let out_path = dir.join(&fname);

        let mut opts = Opts::<S>::default();
        setup(&mut opts);
        for dep_rid in direct_deps(&pf.iface) {
            let dep_hex = hex::encode(dep_rid);
            let val = rewrite_value(backend, &dep_hex);
            if !val.is_empty() {
                opts.rewrites.insert(dep_rid, val);
            }
        }
        fs::write(&out_path, opts.file(&pf.iface))
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", out_path.display()));
    }
}

/// Return `true` if `program` is on PATH.
fn program_on_path(program: &str) -> bool {
    which(program).is_some()
}

fn which(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path).find_map(|dir| {
                let full = dir.join(program);
                if full.is_file() { Some(full) } else { None }
            })
        })
}

/// Run a command and panic with stdout/stderr on failure.
fn run_ok(cmd: &mut Command, label: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{label}: failed to spawn: {e}"));
    if !out.status.success() {
        panic!(
            "{label}: compiler exited {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// C stub for interface99 (allows compile-testing without the real library)
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal stub that satisfies all macros emitted by the C backend.
const INTERFACE99_STUB: &str = r#"/* interface99 stub for pit-gen compile tests */
#pragma once
#include <stdint.h>

/* void* stand-in for untyped resource handles */
typedef void *Any_T;

/* vfunc: emit nothing — we only type-check the #define lines, not expansions */
#define vfunc(ret, name, self, ...) /***/

/* interface(Name): produce a minimal opaque struct so the symbol is defined */
#define interface(Name) \
    typedef struct Name##__vtable { int _unused; } Name##__vtable; \
    typedef struct Name { void *self; Name##__vtable *vptr; } Name;
"#;

// ─────────────────────────────────────────────────────────────────────────────
// C — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_c_type_checks() {
    let compiler = if program_on_path("cc") {
        "cc"
    } else if program_on_path("clang") {
        "clang"
    } else {
        eprintln!("SKIP test_c_type_checks — no C compiler on PATH");
        return;
    };

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<C>(Backend::C, &all, dir, |opts| {
        // Default prefix "P" ensures identifiers start with a letter (C requirement)
        opts.syntax.prefix = "P".into();
    });

    // Write the interface99 stub
    let stub_path = dir.join("interface99.h");
    fs::write(&stub_path, INTERFACE99_STUB).unwrap();

    // Build a test.c that includes all generated headers
    let mut test_c = String::from(
        "/* generated by pit-gen compile test */\n\
         #include \"interface99.h\"\n",
    );
    for (rid, _) in &all {
        let hex = hex::encode(rid);
        test_c.push_str(&format!("#include \"P{hex}.h\"\n"));
    }
    let test_c_path = dir.join("test.c");
    fs::write(&test_c_path, &test_c).unwrap();

    // Compile with -I pointing to the temp dir for stub + generated headers
    run_ok(
        Command::new(compiler)
            .args(["-fsyntax-only", "-std=c11"])
            .arg(format!("-I{}", dir.display()))
            .arg(&test_c_path),
        "C compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Go — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_go_type_checks() {
    if !program_on_path("go") {
        eprintln!("SKIP test_go_type_checks — `go` not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<Go>(Backend::Go, &all, dir, |_| {});

    // Write go.mod
    fs::write(dir.join("go.mod"), "module pitbindings\n\ngo 1.21\n").unwrap();

    // `go build ./...` from the temp dir
    run_ok(
        Command::new("go")
            .args(["build", "./..."])
            .current_dir(dir),
        "Go compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Haxe — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_haxe_type_checks() {
    if !program_on_path("haxe") {
        eprintln!("SKIP test_haxe_type_checks — `haxe` not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    // Haxe expects files in a subdir matching the package name
    let pkg_dir = dir.join("pit");
    fs::create_dir_all(&pkg_dir).unwrap();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<Haxe>(Backend::Haxe, &all, &pkg_dir, |_| {});

    // Build a minimal haxe.hxml that checks all interfaces
    let mut hxml = String::from("--class-path .\n");
    for (rid, _) in &all {
        let hex = hex::encode(rid);
        hxml.push_str(&format!("--library pit.P{hex}\n"));
    }
    hxml.push_str("--no-output\n--interp\n");
    fs::write(dir.join("check.hxml"), &hxml).unwrap();

    run_ok(
        Command::new("haxe").arg("check.hxml").current_dir(dir),
        "Haxe compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript — compile test
// ─────────────────────────────────────────────────────────────────────────────

/// Find the `tsc` binary, checking project node_modules first.
fn find_tsc() -> Option<PathBuf> {
    // 1. Project-local node_modules/.bin/tsc (most reliable)
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let local = manifest.join("../../node_modules/.bin/tsc");
    if local.exists() {
        return Some(local.canonicalize().ok()?);
    }
    // 2. PATH
    which("tsc")
}

#[test]
fn test_typescript_type_checks() {
    let tsc = match find_tsc() {
        Some(p) => p,
        None => {
            eprintln!("SKIP test_typescript_type_checks — `tsc` not found");
            return;
        }
    };

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<TypeScript>(Backend::Ts, &all, dir, |_| {});

    // Write tsconfig.json
    fs::write(
        dir.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true
  },
  "include": ["./*.ts"]
}
"#,
    )
    .unwrap();

    run_ok(
        Command::new(&tsc)
            .arg("--project")
            .arg(dir.join("tsconfig.json")),
        "TypeScript compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Swift — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_swift_type_checks() {
    if !program_on_path("swiftc") {
        eprintln!("SKIP test_swift_type_checks — `swiftc` not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<Swift>(Backend::Swift, &all, dir, |_| {});

    // Collect all .swift files
    let swift_files: Vec<PathBuf> = all
        .keys()
        .map(|rid| dir.join(format!("P{}.swift", hex::encode(rid))))
        .collect();

    // swiftc -typecheck *.swift
    run_ok(
        Command::new("swiftc")
            .arg("-typecheck")
            .args(&swift_files),
        "Swift compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Haskell — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_haskell_type_checks() {
    if !program_on_path("ghc") {
        eprintln!("SKIP test_haskell_type_checks — `ghc` not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let all = parse_all(&collect_pit_files(&pit_dir()));

    // Build rewrites: for each interface, every dep RID → "P<dep_hex>"
    // (so render_file inserts qualified imports).
    for (rid, pf) in &all {
        let hex = hex::encode(rid);
        let fname = format!("P{hex}.hs");
        let out_path = dir.join(&fname);

        let mut opts = Opts::<Haskell>::default();
        for dep_rid in direct_deps(&pf.iface) {
            let dep_hex = hex::encode(dep_rid);
            opts.rewrites.insert(dep_rid, format!("P{dep_hex}"));
        }
        fs::write(&out_path, opts.file(&pf.iface)).unwrap();
    }

    // Collect .hs files — GHC's --make figures out the dependency order
    let hs_files: Vec<PathBuf> = all
        .keys()
        .map(|rid| dir.join(format!("P{}.hs", hex::encode(rid))))
        .collect();

    // ghc --make -fno-code -iDIR *.hs — type-checks without producing .o/.hi files
    run_ok(
        Command::new("ghc")
            .args(["--make", "-fno-code", "-no-link"])
            .arg(format!("-i{}", dir.display()))
            .arg(format!("-outputdir={}", dir.display()))
            .args(&hs_files),
        "Haskell compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rust — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_rust_type_checks() {
    // rustc is always available when running `cargo test`
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let all = parse_all(&collect_pit_files(&pit_dir()));

    // Generate each trait into its own file, then a lib.rs that includes them
    let mut lib_rs = String::from(
        "// Generated by pit-gen compile test\n\
         #![allow(dead_code, unused)]\n\n",
    );
    for (rid, pf) in &all {
        let hex = hex::encode(rid);
        let content = {
            use pit_rust_generic::{Params, interface as rust_iface};
            let params = Params {
                core: syn::parse_quote!(::std),
                flags: Default::default(),
                asyncness: None,
            };
            rust_iface(&params, &pf.iface).to_string()
        };
        // Include everything inline in lib.rs to avoid module path complexity
        lib_rs.push_str(&content);
        lib_rs.push('\n');
    }
    let lib_path = dir.join("lib.rs");
    fs::write(&lib_path, &lib_rs).unwrap();

    // Compile with rustc directly
    run_ok(
        Command::new("rustc")
            .args(["--edition", "2021", "--crate-type", "lib"])
            .arg(&lib_path),
        "Rust compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Java — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_java_type_checks() {
    if !program_on_path("javac") {
        eprintln!("SKIP test_java_type_checks — javac not on PATH");
        return;
    }
    if !Command::new("javac")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        eprintln!("SKIP test_java_type_checks — javac not functional");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let pkg_dir = dir.join("pc/portal/pit/guest");
    fs::create_dir_all(&pkg_dir).unwrap();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<Java>(Backend::Java, &all, &pkg_dir, |_| {});

    let java_files: Vec<PathBuf> = all
        .keys()
        .map(|rid| pkg_dir.join(format!("P{}.java", hex::encode(rid))))
        .collect();

    run_ok(
        Command::new("javac").args(&java_files),
        "Java compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scala — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_scala_type_checks() {
    if !program_on_path("scalac") {
        eprintln!("SKIP test_scala_type_checks — scalac not on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let pkg_dir = dir.join("pc/portal/pit/guest/scala");
    fs::create_dir_all(&pkg_dir).unwrap();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<Scala>(Backend::Scala, &all, &pkg_dir, |_| {});

    let scala_files: Vec<PathBuf> = all
        .keys()
        .map(|rid| pkg_dir.join(format!("P{}.scala", hex::encode(rid))))
        .collect();

    run_ok(
        Command::new("scalac").args(&scala_files),
        "Scala compile test",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript async — compile test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_typescript_async_type_checks() {
    let tsc = match find_tsc() {
        Some(p) => p,
        None => {
            eprintln!("SKIP test_typescript_async_type_checks — tsc not found");
            return;
        }
    };

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let all = parse_all(&collect_pit_files(&pit_dir()));
    generate_syntax::<TypeScriptAsync>(Backend::TsAsync, &all, dir, |_| {});

    fs::write(
        dir.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true
  },
  "include": ["./*.ts"]
}
"#,
    )
    .unwrap();

    run_ok(
        Command::new(&tsc)
            .arg("--project")
            .arg(dir.join("tsconfig.json")),
        "TypeScript async compile test",
    );
}
