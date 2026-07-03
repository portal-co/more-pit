//! Compile tests: canonical `impl/` against pit-gen generated APIs.
//! Policy: [`docs/compile-tests.md`](../../../../docs/compile-tests.md)

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use pit_core::parse_interface;
use pit_lang_generic::{Go, Opts, TypeScript, Syntax};
use pit_js_teavm::{JsTeavmBackend, JsTeavmContext};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn impl_dir() -> PathBuf {
    repo_root().join("impl")
}

fn buffer_pit() -> PathBuf {
    repo_root().join("pit/common/buffer.pit")
}

fn run_ok(cmd: &mut Command, label: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("{label}: failed to spawn: {e}"));
    if !out.status.success() {
        panic!(
            "{label}: exited {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

#[test]
fn rust_impl_compiles_with_generated_trait() {
    run_ok(
        Command::new("cargo")
            .args(["test", "-p", "pit-impl-rust-fixture"])
            .current_dir(repo_root()),
        "rust impl nested cargo test",
    );
}

fn require_toolchain(program: &str, install_hint: &str) -> PathBuf {
    find_tsc_like(program).unwrap_or_else(|| {
        panic!(
            "compile test requires `{program}` on PATH — {install_hint}\n\
             See docs/compile-tests.md"
        )
    })
}

fn find_tsc_like(program: &str) -> Option<PathBuf> {
    if program == "tsc" {
        let local = repo_root().join("node_modules/.bin/tsc");
        if local.exists() {
            return local.canonicalize().ok();
        }
    }
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path).find_map(|dir| {
            let full = dir.join(program);
            if full.is_file() { Some(full) } else { None }
        })
    })
}

#[test]
fn typescript_impl_type_checks() {
    let tsc = require_toolchain("tsc", "run `npm ci` in more-pit or install typescript globally");

    let pit_src = fs::read_to_string(buffer_pit()).expect("buffer.pit");
    let (_, iface) = parse_interface(&pit_src).expect("parse buffer.pit");
    let generated = Opts::<TypeScript>::default().file(&iface);
    let hex = hex::encode(iface.rid());

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(dir.join(format!("P{hex}.ts")), generated).unwrap();
    fs::copy(
        impl_dir().join("ts/buffer_slice.ts"),
        dir.join("buffer_slice.ts"),
    )
    .unwrap();

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
        "typescript impl compile test",
    );
}

#[test]
fn js_teavm_roundtrip_fixture_type_checks() {
    let tsc = require_toolchain("tsc", "run `npm ci` in more-pit or install typescript globally");

    let pit_src = fs::read_to_string(buffer_pit()).expect("buffer.pit");
    let (_, iface) = parse_interface(&pit_src).expect("parse buffer.pit");
    let hex = hex::encode(iface.rid());

    let tmp = tempfile::tempdir().unwrap();
    let ts_dir = tmp.path().join("ts");
    fs::create_dir_all(&ts_dir).unwrap();

    let ctx = JsTeavmContext::default();
    for file in JsTeavmBackend::emit_shared_files(&ctx) {
        if file.path.starts_with("ts/") {
            fs::write(ts_dir.join(file.path.strip_prefix("ts/").unwrap()), file.content).unwrap();
        }
    }
    fs::write(
        ts_dir.join(format!("P{hex}.ts")),
        Opts::<TypeScript>::default().file(&iface),
    )
    .unwrap();
    for file in JsTeavmBackend::emit_interface_files(&iface, &ctx) {
        if file.path.starts_with("ts/") {
            fs::write(ts_dir.join(file.path.strip_prefix("ts/").unwrap()), file.content).unwrap();
        }
    }

    let mut fixture = fs::read_to_string(impl_dir().join("ts/buffer_slice.ts")).unwrap();
    fixture = fixture.replace(
        &format!("./P{hex}.js"),
        &format!("./P{hex}"),
    );
    fs::write(ts_dir.join("buffer_slice.ts"), fixture).unwrap();

    fs::write(
        ts_dir.join("tsconfig.json"),
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
            .arg(ts_dir.join("tsconfig.json")),
        "js-teavm impl fixture compile test",
    );
}

#[test]
fn go_impl_compiles_and_runs() {
    let _go = require_toolchain("go", "install Go from https://go.dev/dl/");

    let pit_src = fs::read_to_string(buffer_pit()).expect("buffer.pit");
    let (_, iface) = parse_interface(&pit_src).expect("parse buffer.pit");
    let generated = Opts::<Go>::default().file(&iface);
    let rid = iface.rid_str();

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(dir.join("go.mod"), "module pitimpltest\n\ngo 1.22\n").unwrap();
    fs::write(dir.join(format!("P{rid}.go")), generated).unwrap();

    let mut go_impl = fs::read_to_string(impl_dir().join("go/buffer_slice.go")).unwrap();
    go_impl = go_impl.replace("package buffer", "package pitbindings");
    fs::write(dir.join("buffer_slice.go"), go_impl).unwrap();

    let test_go = format!(
        r#"package pitbindings

import "testing"

func TestSliceBuffer(t *testing.T) {{
    s := &SliceBuffer{{Data: []byte{{1, 2, 3}}}}
    if got := s.P{rid}_read8(1); got != 2 {{
        t.Fatalf("read8: got %d want 2", got)
    }}
    s.P{rid}_write8(0, 9)
    if s.Data[0] != 9 {{
        t.Fatalf("write8 failed")
    }}
    if got := s.P{rid}_size(); got != 3 {{
        t.Fatalf("size: got %d want 3", got)
    }}
}}
"#,
    );
    fs::write(dir.join("buffer_slice_test.go"), test_go).unwrap();

    run_ok(
        Command::new("go").args(["test", "./..."]).current_dir(dir),
        "go impl compile test",
    );
}

#[test]
fn resource_dep_interfaces_compile_rust() {
    run_ok(
        Command::new("cargo")
            .args(["check", "-p", "pit-resource-deps-fixture"])
            .current_dir(repo_root()),
        "rust resource-dep nested cargo check",
    );
}
