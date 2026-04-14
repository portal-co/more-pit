//! `pit-gen` binary — CLI entry point.
//!
//! All shared logic lives in [`pit_gen`] (the library crate).
//! This file contains only argument parsing and top-level orchestration.

use std::{
    collections::BTreeMap,
    fmt::Write as FmtWrite,
    fs,
    path::{Path, PathBuf},
    process,
};

use pit_core::parse_interface;
use pit_gen::{Backend, direct_deps, output_filename, rewrite_value};
use pit_lang_generic::{C, Go, Haskell, Haxe, Opts, Swift, Syntax, TypeScript, TypeScriptAsync};

// ─────────────────────────────────────────────────────────────────────────────
// Config
// ─────────────────────────────────────────────────────────────────────────────

struct Config {
    backend: Backend,
    out_dir: PathBuf,
    pit_dirs: Vec<PathBuf>,
    files: Vec<PathBuf>,
    c_prefix: String,
}

fn print_help() {
    eprintln!(
        "pit-gen -- PIT interface code-generator\n\
         \n\
         USAGE:\n\
         \n\
         \tpit-gen --backend <BACKEND> [OPTIONS] [FILE...]\n\
         \n\
         OPTIONS:\n\
         \n\
         \t--backend <BACKEND>  Target language (required)\n\
         \t                       c | go | haxe | ts | ts-async | swift | haskell | rust\n\
         \t--out-dir <DIR>      Output directory  [default: ./generated/<backend>]\n\
         \t--pit-dir <DIR>      Scan directory for *.pit files (repeatable)\n\
         \t--prefix <PREFIX>    C type-name prefix  [default: \"\"]\n\
         \t--help               Print this message\n\
         \n\
         Each .pit file is compiled to one output file in OUT_DIR.\n\
         Dependencies between interfaces are resolved automatically.\n"
    );
}

fn parse_args() -> Config {
    let mut args = std::env::args().skip(1).peekable();
    let mut backend: Option<Backend> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut pit_dirs: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();
    let mut c_prefix = String::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            "--backend" => {
                let val = args.next().unwrap_or_else(|| {
                    eprintln!("pit-gen: --backend requires an argument");
                    process::exit(1);
                });
                backend = Some(Backend::parse(&val).unwrap_or_else(|| {
                    eprintln!("pit-gen: unknown backend '{val}'");
                    process::exit(1);
                }));
            }
            "--out-dir" => {
                out_dir = Some(PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("pit-gen: --out-dir requires a path");
                    process::exit(1);
                })));
            }
            "--pit-dir" => {
                pit_dirs.push(PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("pit-gen: --pit-dir requires a path");
                    process::exit(1);
                })));
            }
            "--prefix" => {
                c_prefix = args.next().unwrap_or_else(|| {
                    eprintln!("pit-gen: --prefix requires a string");
                    process::exit(1);
                });
            }
            other if other.starts_with("--") => {
                eprintln!("pit-gen: unknown flag '{other}'");
                process::exit(1);
            }
            path => {
                files.push(PathBuf::from(path));
            }
        }
    }

    let backend = backend.unwrap_or_else(|| {
        eprintln!("pit-gen: --backend is required");
        print_help();
        process::exit(1);
    });

    let out_dir = out_dir
        .unwrap_or_else(|| PathBuf::from("generated").join(backend.name()));

    Config { backend, out_dir, pit_dirs, files, c_prefix }
}

// ─────────────────────────────────────────────────────────────────────────────
// .pit file discovery
// ─────────────────────────────────────────────────────────────────────────────

fn collect_pit_files(cfg: &Config) -> Vec<PathBuf> {
    let mut out = cfg.files.clone();
    for dir in &cfg.pit_dirs {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("pit-gen: cannot read directory '{}': {e}", dir.display());
                process::exit(1);
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("pit") {
                out.push(path);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Parse all .pit files → RID index
// ─────────────────────────────────────────────────────────────────────────────

struct PitFile {
    #[allow(dead_code)]
    path: PathBuf,
    iface: pit_core::Interface,
}

fn parse_all(paths: &[PathBuf]) -> BTreeMap<[u8; 32], PitFile> {
    let mut map: BTreeMap<[u8; 32], PitFile> = BTreeMap::new();
    for path in paths {
        let src = fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("pit-gen: cannot read '{}': {e}", path.display());
            process::exit(1);
        });
        let iface = match parse_interface(&src) {
            Ok((_, iface)) => iface,
            Err(e) => {
                eprintln!(
                    "pit-gen: skipping '{}' (parse error: {})",
                    path.display(),
                    e
                );
                continue;
            }
        };
        let rid = iface.rid();
        map.insert(rid, PitFile { path: path.clone(), iface });
    }
    map
}

// ─────────────────────────────────────────────────────────────────────────────
// Generic generation dispatch
// ─────────────────────────────────────────────────────────────────────────────

fn generate_with_syntax<S: Syntax>(
    cfg: &Config,
    all: &BTreeMap<[u8; 32], PitFile>,
    setup_syntax: impl Fn(&mut Opts<S>),
) {
    fs::create_dir_all(&cfg.out_dir).unwrap_or_else(|e| {
        eprintln!("pit-gen: cannot create '{}': {e}", cfg.out_dir.display());
        process::exit(1);
    });

    for (rid, pf) in all {
        let hex = hex::encode(rid);
        let fname = output_filename(cfg.backend, &hex);
        let out_path = cfg.out_dir.join(&fname);

        let mut opts = Opts::<S>::default();
        setup_syntax(&mut opts);
        for dep_rid in direct_deps(&pf.iface) {
            let dep_hex = hex::encode(dep_rid);
            let val = rewrite_value(cfg.backend, &dep_hex);
            if !val.is_empty() {
                opts.rewrites.insert(dep_rid, val);
            }
        }

        let content = opts.file(&pf.iface);
        fs::write(&out_path, &content).unwrap_or_else(|e| {
            eprintln!("pit-gen: cannot write '{}': {e}", out_path.display());
            process::exit(1);
        });
        eprintln!("  wrote {}", out_path.display());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rust backend
// ─────────────────────────────────────────────────────────────────────────────

fn generate_rust_file(iface: &pit_core::Interface) -> String {
    use pit_rust_generic::{Params, interface as rust_iface};
    let params = Params {
        core: syn::parse_quote!(::std),
        flags: Default::default(),
        asyncness: None,
    };
    rust_iface(&params, iface).to_string()
}

fn generate_rust_lib(rids: &[[u8; 32]]) -> String {
    let mut out = String::from("// Generated by pit-gen — do not edit manually.\n\n");
    for rid in rids {
        let hex = hex::encode(rid);
        let _ = writeln!(out, "pub mod p_{hex};");
    }
    out
}

fn generate_rust(cfg: &Config, all: &BTreeMap<[u8; 32], PitFile>) {
    fs::create_dir_all(&cfg.out_dir).unwrap_or_else(|e| {
        eprintln!("pit-gen: cannot create '{}': {e}", cfg.out_dir.display());
        process::exit(1);
    });

    for (rid, pf) in all {
        let hex = hex::encode(rid);
        let out_path = cfg.out_dir.join(output_filename(cfg.backend, &hex));
        fs::write(&out_path, generate_rust_file(&pf.iface)).unwrap_or_else(|e| {
            eprintln!("pit-gen: cannot write '{}': {e}", out_path.display());
            process::exit(1);
        });
        eprintln!("  wrote {}", out_path.display());
    }

    let lib_path = cfg.out_dir.join("lib.rs");
    let rids: Vec<[u8; 32]> = all.keys().copied().collect();
    fs::write(&lib_path, generate_rust_lib(&rids)).ok();
    eprintln!("  wrote {}", lib_path.display());
}

// ─────────────────────────────────────────────────────────────────────────────
// Scaffold files
// ─────────────────────────────────────────────────────────────────────────────

fn write_scaffold(backend: Backend, out_dir: &Path, _all: &BTreeMap<[u8; 32], PitFile>) {
    match backend {
        Backend::Go => {
            let p = out_dir.join("go.mod");
            if !p.exists() {
                fs::write(&p, "module pitbindings\n\ngo 1.21\n").ok();
                eprintln!("  wrote {}", p.display());
            }
        }
        Backend::Ts | Backend::TsAsync => {
            let p = out_dir.join("tsconfig.json");
            if !p.exists() {
                fs::write(
                    &p,
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
                .ok();
                eprintln!("  wrote {}", p.display());
            }
        }
        _ => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let cfg = parse_args();

    let pit_files = collect_pit_files(&cfg);
    if pit_files.is_empty() {
        eprintln!("pit-gen: no .pit files specified (use --pit-dir or pass files directly)");
        process::exit(1);
    }

    eprintln!(
        "pit-gen: backend={}  out={}  ({} interface(s))",
        cfg.backend.name(),
        cfg.out_dir.display(),
        pit_files.len(),
    );

    let all = parse_all(&pit_files);

    match cfg.backend {
        Backend::C => {
            // Default prefix "P" ensures generated type names start with a letter
            // (C identifiers may not begin with a digit).
            let prefix = if cfg.c_prefix.is_empty() {
                "P".to_string()
            } else {
                cfg.c_prefix.clone()
            };
            generate_with_syntax::<C>(&cfg, &all, move |opts| {
                opts.syntax.prefix = prefix.clone();
            });
        }
        Backend::Go => generate_with_syntax::<Go>(&cfg, &all, |_| {}),
        Backend::Haxe => generate_with_syntax::<Haxe>(&cfg, &all, |_| {}),
        Backend::Ts => generate_with_syntax::<TypeScript>(&cfg, &all, |_| {}),
        Backend::TsAsync => generate_with_syntax::<TypeScriptAsync>(&cfg, &all, |_| {}),
        Backend::Swift => generate_with_syntax::<Swift>(&cfg, &all, |_| {}),
        Backend::Haskell => generate_with_syntax::<Haskell>(&cfg, &all, |_| {}),
        Backend::Rust => generate_rust(&cfg, &all),
    }

    write_scaffold(cfg.backend, &cfg.out_dir, &all);
    eprintln!("pit-gen: done.");
}
