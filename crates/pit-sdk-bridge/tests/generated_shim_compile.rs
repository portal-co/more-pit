use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use pit_sdk_bridge::{emit_rust_shim, emit_typescript_shim, lower_sdk};
use portal_solutions_sdk::{
    Arity, Sdk, SdkInterface, SdkItem, SdkItemContents, SdkMethod, SdkParam, SdkTy,
};

type AnyError = Box<dyn std::error::Error>;

fn param(ty: SdkTy) -> SdkParam {
    SdkParam {
        ty,
        attr: Vec::new(),
    }
}

fn fixture_documents() -> BTreeMap<String, Sdk> {
    let method = SdkMethod {
        arity: Arity::default(),
        args: BTreeMap::from([
            (
                "blob".to_owned(),
                param(SdkTy::Array {
                    item: Box::new(param(SdkTy::I32)),
                }),
            ),
            (
                "child".to_owned(),
                param(SdkTy::Path {
                    sdk: None,
                    ty: "Child".to_owned(),
                    args: BTreeMap::new(),
                }),
            ),
            ("octet".to_owned(), param(SdkTy::Byte)),
        ]),
        rets: BTreeMap::from([("small".to_owned(), param(SdkTy::I16))]),
        attr: Vec::new(),
    };
    let interface = SdkInterface {
        methods: BTreeMap::from([("echo".to_owned(), (Arity::default(), method))]),
        attr: Vec::new(),
    };
    BTreeMap::from([(
        "fixture.sdk".to_owned(),
        Sdk {
            interfaces: BTreeMap::from([
                (
                    "Api".to_owned(),
                    SdkItem {
                        generics: Arity::default(),
                        contents: SdkItemContents::Type(param(SdkTy::Interface {
                            implementation: interface,
                        })),
                    },
                ),
                (
                    "Child".to_owned(),
                    SdkItem {
                        generics: Arity::default(),
                        contents: SdkItemContents::Type(param(SdkTy::Interface {
                            implementation: SdkInterface {
                                methods: BTreeMap::new(),
                                attr: Vec::new(),
                            },
                        })),
                    },
                ),
            ]),
        },
    )])
}

fn fixture() -> pit_sdk_bridge::LoweredSdk {
    lower_sdk("fixture.sdk", &fixture_documents()).unwrap()
}

fn run(program: &Path, args: &[&str], cwd: &Path, label: &str) {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("{label}: failed to start {}: {error}", program.display()));
    assert!(
        output.status.success(),
        "{label} failed ({})\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn find_tool(program: &str, hint: &str) -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("compile test requires `{program}` on PATH — {hint}"))
}

#[test]
fn sdk_cli_generates_only_supported_shim_backends() -> Result<(), AnyError> {
    let temp = tempfile::tempdir()?;
    let dir = temp.path();
    let source = "Api <>{echo <><>(blob array i32,octet byte,):(small i16,)}";
    let (remaining, _) = Sdk::parse(source).unwrap();
    assert!(
        remaining.trim().is_empty(),
        "unparsed SDK input: {remaining:?}"
    );
    fs::write(dir.join("fixture.sdk"), source)?;
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_pit-sdk-gen"));
    run(
        &binary,
        &[
            "--root",
            "fixture.sdk",
            "--backend",
            "ts",
            "--out",
            "generated.ts",
        ],
        dir,
        "pit-sdk-gen CLI",
    );
    let generated = fs::read_to_string(dir.join("generated.ts"))?;
    assert!(generated.contains("sdkArgsToPit"));
    assert!(generated.contains("pitReturnsToSdk"));

    run(
        &binary,
        &[
            "--root",
            "fixture.sdk",
            "--backend",
            "rust",
            "--out",
            "generated.rs",
        ],
        dir,
        "pit-sdk-gen Rust CLI",
    );
    let generated_rust = fs::read_to_string(dir.join("generated.rs"))?;
    assert!(generated_rust.contains("sdk_args_to_pit"));
    assert!(generated_rust.contains("pit_returns_to_sdk"));

    let rejected = Command::new(&binary)
        .args([
            "--root",
            "fixture.sdk",
            "--backend",
            "go",
            "--out",
            "unused.go",
        ])
        .current_dir(dir)
        .output()?;
    assert!(
        !rejected.status.success(),
        "unsupported shim backend must fail"
    );
    assert!(
        !dir.join("unused.go").exists(),
        "failure must not write partial output"
    );
    Ok(())
}

#[test]
fn generated_rust_shim_type_checks() -> Result<(), AnyError> {
    let generated = emit_rust_shim(&fixture())?;
    let temp = tempfile::tempdir()?;
    let crate_dir = temp.path();
    fs::create_dir_all(crate_dir.join("src"))?;
    let bridge_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"generated-rust-shim-check\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\npit-sdk-bridge = {{ path = {:?}, features = [\"unstable-sdk\"] }}\n",
            bridge_path
        ),
    )?;
    fs::write(crate_dir.join("src/lib.rs"), "pub mod generated;\n")?;
    fs::write(crate_dir.join("src/generated.rs"), generated)?;
    run(
        &find_tool("cargo", "install Rust and Cargo"),
        &[
            "check",
            "--offline",
            "--manifest-path",
            crate_dir.join("Cargo.toml").to_str().unwrap(),
        ],
        crate_dir,
        "generated Rust shim compile test",
    );
    Ok(())
}

#[test]
fn generated_typescript_shim_type_checks_and_round_trips() -> Result<(), AnyError> {
    let tsc = find_tool(
        "tsc",
        "install TypeScript globally or run npm ci in more-pit",
    );
    let node = find_tool(
        "node",
        "install Node.js to execute the TypeScript round-trip fixture",
    );
    let temp = tempfile::tempdir()?;
    let dir = temp.path();
    fs::write(dir.join("sdk_shim.ts"), emit_typescript_shim(&fixture())?)?;
    fs::write(dir.join("package.json"), r#"{ "type": "module" }"#)?;
    fs::write(
        dir.join("test.ts"),
        r##"import {
  PitValue,
  ResourceResolver,
  SdkValue,
  pitArgsToSdk,
  pitReturnsToSdk,
  plans,
  sdkArgsToPit,
  sdkReturnsToPit,
} from "./sdk_shim.js";

function assert(condition: boolean, message: string): void {
  if (!condition) throw new Error(message);
}
const source = plans.find((plan) => plan.methods.echo !== undefined)!.sourceIdentity;
const stored = new Map<number, string>();
const interfaces = new Map<number, string>();
const interfaceTargets = new Map<number, { identity: string; rid: string | null }>();
let nextHandle = 1;
const resolver: ResourceResolver<string> = {
  storeAny(_owner, _sourceType, _targetIdentity, value) {
    const handle = nextHandle++;
    stored.set(handle, value);
    return handle;
  },
  loadAny(_owner, _sourceType, _targetIdentity, handle) {
    const value = stored.get(handle);
    if (value === undefined) throw new Error("unknown any handle");
    return value;
  },
  storeInterface(identity, rid, value) {
    const handle = nextHandle++;
    interfaces.set(handle, value);
    interfaceTargets.set(handle, { identity, rid });
    return handle;
  },
  loadInterface(identity, rid, handle) {
    const target = interfaceTargets.get(handle);
    if (target?.identity !== identity || target.rid !== rid) throw new Error("interface target metadata changed");
    const value = interfaces.get(handle);
    if (value === undefined) throw new Error("unknown interface handle");
    return value;
  },
};
const pitArgs = sdkArgsToPit(source, "echo", [
  { kind: "any", value: "opaque aggregate" },
  { kind: "interface", value: "child object" },
  { kind: "byte", value: 255 },
], resolver);
assert(pitArgs[0]?.kind === "resource", "aggregate should become a resource");
assert(pitArgs[1]?.kind === "resource", "interface should become a resource");
assert(pitArgs[2]?.kind === "i32" && pitArgs[2].value === 255, "byte carrier should be I32");
const sdkArgs = pitArgsToSdk(source, "echo", pitArgs, resolver);
assert(sdkArgs[0]?.kind === "any" && sdkArgs[0].value === "opaque aggregate", "aggregate should round-trip");
assert(sdkArgs[1]?.kind === "interface" && sdkArgs[1].value === "child object", "interface should round-trip");
const childTarget = [...interfaceTargets.values()][0]!;
assert(childTarget.identity.includes("#Child"), "interface target identity should be preserved");
assert(childTarget.rid !== null, "interface target RID should be preserved");
assert(sdkArgs[2]?.kind === "byte" && sdkArgs[2].value === 255, "byte should round-trip");
const pitReturns = sdkReturnsToPit(source, "echo", [{ kind: "i16", value: -32768 }], resolver);
assert(pitReturns[0]?.kind === "i32" && pitReturns[0].value === -32768, "i16 carrier should be I32");
const sdkReturns = pitReturnsToSdk(source, "echo", pitReturns, resolver);
assert(sdkReturns[0]?.kind === "i16" && sdkReturns[0].value === -32768, "i16 should round-trip");
let rangeRejected = false;
try {
  sdkArgsToPit(source, "echo", [
    { kind: "any", value: "x" },
    { kind: "interface", value: "child" },
    { kind: "byte", value: 256 },
  ], resolver);
} catch {
  rangeRejected = true;
}
assert(rangeRejected, "out-of-range byte must fail instead of truncating");
const wrongWireValue: PitValue = { kind: "i32", value: 32768 };
let returnRangeRejected = false;
try {
  pitReturnsToSdk(source, "echo", [wrongWireValue], resolver);
} catch {
  returnRangeRejected = true;
}
assert(returnRangeRejected, "out-of-range i16 must fail instead of truncating");
"##,
    )?;
    fs::write(
        dir.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "noEmit": false,
    "outDir": "dist"
  },
  "include": ["./*.ts"]
}
"#,
    )?;
    run(
        &tsc,
        &[
            "--project",
            dir.join("tsconfig.json").to_str().unwrap(),
            "--pretty",
            "false",
        ],
        dir,
        "generated TypeScript shim compile test",
    );
    run(
        &node,
        &[dir.join("dist/test.js").to_str().unwrap()],
        dir,
        "generated TypeScript shim round-trip test",
    );
    Ok(())
}
