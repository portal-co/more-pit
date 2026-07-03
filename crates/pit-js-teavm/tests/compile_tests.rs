//! pit-js-teavm compile tests.
//!
//! Policy: [`docs/compile-tests.md`](../../docs/compile-tests.md)

use std::{
    fs,
    path::PathBuf,
    process::Command,
};

use pit_core::{parse_interface, Interface};
use pit_js_teavm::{JsTeavmBackend, JsTeavmContext, DEFAULT_JAVA_PKG, DEFAULT_SCALA_PKG};
use pit_lang_generic::{Java, Opts, Scala, Syntax, TypeScript};
use pit_gen::direct_deps;

fn require_toolchain(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path).find_map(|dir| {
                let full = dir.join(name);
                if full.is_file() { Some(full) } else { None }
            })
        })
        .unwrap_or_else(|| {
            panic!(
                "required toolchain `{name}` not found on PATH; see docs/compile-tests.md"
            )
        })
}

fn require_tsc() -> PathBuf {
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../node_modules/.bin/tsc");
    if local.is_file() {
        return local;
    }
    require_toolchain("tsc")
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

const BUFFER: &str = r#"{
    read8(I32) -> (I32);
    write8(I32,I32) -> ();
    size() -> (I32)
}"#;

const READER: &str = r#"{
    read(I32) -> (R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5);
    read64(I64) -> (R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d)
}"#;

#[test]
fn java_glue_emits_js_export_not_tpit() {
    let (_, iface) = parse_interface(BUFFER).unwrap();
    let files = JsTeavmBackend::emit_interface_files(&iface, &JsTeavmContext::default());
    let java = files
        .iter()
        .find(|f| f.path.ends_with("Js.java"))
        .expect("P{rid}Js.java");
    assert!(java.content.contains("ExportObject"));
    assert!(java.content.contains("JsProxy"));
    assert!(java.content.contains("@JSExport"));
    assert!(!java.content.contains("tpit/"));
}

#[test]
fn js_teavm_java_compiles_with_javac() {
    let _javac = require_toolchain("javac");
    let (_, iface) = parse_interface(BUFFER).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pkg_path = dir.path().join(DEFAULT_JAVA_PKG.replace('.', "/"));
    let jso_path = dir.path().join("org/teavm/jso");
    fs::create_dir_all(&pkg_path).unwrap();
    fs::create_dir_all(&jso_path).unwrap();

    for name in ["JSBody.java", "JSExport.java", "JSObject.java"] {
        fs::copy(
            fixture_root().join(format!("java/org/teavm/jso/{name}")),
            jso_path.join(name),
        )
        .unwrap();
    }

    let ctx = JsTeavmContext::default();
    let mut sources = vec![
        jso_path.join("JSBody.java"),
        jso_path.join("JSExport.java"),
        jso_path.join("JSObject.java"),
    ];

    for file in JsTeavmBackend::emit_shared_files(&ctx) {
        if !file.path.starts_with("java/") {
            continue;
        }
        let path = pkg_path.join(file.path.strip_prefix("java/").unwrap());
        fs::write(&path, file.content).unwrap();
        sources.push(path);
    }

    let rid = iface.rid_str();
    let canonical_path = pkg_path.join(format!("P{rid}.java"));
    fs::write(
        &canonical_path,
        Opts::<Java>::default().file(&iface),
    )
    .unwrap();
    sources.push(canonical_path);

    for file in JsTeavmBackend::emit_interface_files(&iface, &ctx) {
        if !file.path.starts_with("java/") {
            continue;
        }
        let path = pkg_path.join(file.path.strip_prefix("java/").unwrap());
        fs::write(&path, file.content).unwrap();
        sources.push(path);
    }

    let status = Command::new("javac")
        .arg("-cp")
        .arg(dir.path())
        .args(&sources)
        .status()
        .expect("javac");
    assert!(status.success(), "javac failed for pit-js-teavm Java glue");
}

#[test]
fn js_teavm_scala_compiles_with_scalac() {
    let _scalac = require_toolchain("scalac");
    let (_, iface) = parse_interface(BUFFER).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pkg_path = dir.path().join(DEFAULT_SCALA_PKG.replace('.', "/"));
    let jso_path = dir.path().join("org/teavm/jso");
    fs::create_dir_all(&pkg_path).unwrap();
    fs::create_dir_all(&jso_path).unwrap();

    fs::copy(
        fixture_root().join("scala/org/teavm/jso/JsoStubs.scala"),
        jso_path.join("JsoStubs.scala"),
    )
    .unwrap();

    let ctx = JsTeavmContext::default();
    let mut scala_sources = vec![jso_path.join("JsoStubs.scala")];

    for file in JsTeavmBackend::emit_shared_files(&ctx) {
        if !file.path.starts_with("scala/") {
            continue;
        }
        let path = pkg_path.join(file.path.strip_prefix("scala/").unwrap());
        fs::write(&path, file.content).unwrap();
        scala_sources.push(path);
    }

    let rid = iface.rid_str();
    let canonical_path = pkg_path.join(format!("P{rid}.scala"));
    fs::write(
        &canonical_path,
        Opts::<Scala>::default().file(&iface),
    )
    .unwrap();
    scala_sources.push(canonical_path);

    for file in JsTeavmBackend::emit_interface_files(&iface, &ctx) {
        if !file.path.starts_with("scala/") {
            continue;
        }
        let path = pkg_path.join(file.path.strip_prefix("scala/").unwrap());
        fs::write(&path, file.content).unwrap();
        scala_sources.push(path);
    }

    let status = Command::new("scalac")
        .arg("-classpath")
        .arg(dir.path())
        .args(&scala_sources)
        .status()
        .expect("scalac");
    assert!(status.success(), "scalac failed for pit-js-teavm Scala glue");
}

#[test]
fn js_teavm_ts_type_checks() {
    let tsc = require_tsc();
    let (_, buffer) = parse_interface(BUFFER).unwrap();
    let (_, reader) = parse_interface(READER).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ts_dir = dir.path().join("ts");
    fs::create_dir_all(&ts_dir).unwrap();

    let ctx = JsTeavmContext::default();
    for file in JsTeavmBackend::emit_shared_files(&ctx) {
        if file.path.starts_with("ts/") {
            fs::write(ts_dir.join(file.path.strip_prefix("ts/").unwrap()), file.content).unwrap();
        }
    }

    // reader depends on buffer and buffer64 — emit dependency glue
    let buffer_src = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../pit/common/buffer.pit"),
    )
    .unwrap();
    let buffer64_src = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../pit/common/buffer64.pit"),
    )
    .unwrap();
    let (_, buffer_iface) = parse_interface(&buffer_src).unwrap();
    let (_, buffer64_iface) = parse_interface(&buffer64_src).unwrap();
    for dep_iface in [&buffer_iface, &buffer64_iface] {
        let dep_rid = dep_iface.rid_str();
        if ts_dir.join(format!("P{dep_rid}_js.ts")).exists() {
            continue;
        }
        fs::write(
            ts_dir.join(format!("P{dep_rid}.ts")),
            Opts::<TypeScript>::default().file(dep_iface),
        )
        .unwrap();
        for file in JsTeavmBackend::emit_interface_files(dep_iface, &ctx) {
            if file.path.starts_with("ts/") {
                fs::write(ts_dir.join(file.path.strip_prefix("ts/").unwrap()), file.content).unwrap();
            }
        }
    }

    for iface in [&buffer, &reader] {
        let rid = iface.rid_str();
        let mut opts = Opts::<TypeScript>::default();
        for dep_rid in direct_deps(iface) {
            let dep_hex = hex::encode(dep_rid);
            opts.rewrites.insert(dep_rid, format!("./P{dep_hex}"));
        }
        fs::write(
            ts_dir.join(format!("P{rid}.ts")),
            opts.file(iface),
        )
        .unwrap();
        for file in JsTeavmBackend::emit_interface_files(iface, &ctx) {
            if file.path.starts_with("ts/") {
                fs::write(ts_dir.join(file.path.strip_prefix("ts/").unwrap()), file.content).unwrap();
            }
        }
    }

    // reader depends on buffer — emit buffer adapter for imports
    let buffer_rid = buffer.rid_str();
    let reader_rid = reader.rid_str();
    let reader_js = fs::read_to_string(ts_dir.join(format!("P{reader_rid}_js.ts"))).unwrap();
    assert!(
        reader_js.contains(&format!("P{buffer_rid}_js")),
        "reader adapter should import buffer peer glue"
    );

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

    let status = Command::new(&tsc)
        .arg("--project")
        .arg(ts_dir.join("tsconfig.json"))
        .status()
        .expect("tsc");
    assert!(status.success(), "tsc failed for pit-js-teavm TS glue");
}
