use pit_core::parse_interface;
use pit_rust_generic::{interface as rust_iface, FeatureFlags, Params};
use quote::quote;
use std::{env, fs, path::PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn main() {
    let pit_dir = repo_root().join("pit/common");
    let names = ["buffer.pit", "buffer64.pit", "reader.pit", "writer.pit"];
    let mut lib = String::new();
    for name in names {
        let path = pit_dir.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        let src = fs::read_to_string(&path).expect(name);
        let (_, iface) = parse_interface(&src).expect(name);
        let tokens = rust_iface(
            &Params {
                core: syn::parse_quote!(::std),
                flags: FeatureFlags::default(),
                asyncness: None,
            },
            &iface,
        );
        lib.push_str(&tokens.to_string());
        lib.push('\n');
    }
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("fixture.rs"),
        lib,
    )
    .expect("write fixture");
}
