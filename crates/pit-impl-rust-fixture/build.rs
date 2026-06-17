use pit_core::parse_interface;
use pit_rust_generic::{interface as rust_iface, FeatureFlags, Params};
use std::{env, fs, path::PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn buffer_pit() -> PathBuf {
    repo_root().join("pit/common/buffer.pit")
}

fn sanitize_rust_impl(src: &str) -> String {
    let mut out = String::new();
    let mut lines = src.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("pub trait P") && line.contains("<'bound>") {
            let mut depth =
                line.matches('{').count() as i32 - line.matches('}').count() as i32;
            while depth > 0 {
                let next = lines.next().unwrap_or("");
                depth += next.matches('{').count() as i32;
                depth -= next.matches('}').count() as i32;
            }
            continue;
        }
        if line.starts_with("#[cfg(test)]") {
            break;
        }
        let line = line
            .strip_prefix("//!")
            .map(|s| format!("//{s}"))
            .unwrap_or_else(|| line.to_string());
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn main() {
    let pit = buffer_pit();
    println!("cargo:rerun-if-changed={}", pit.display());
    let impl_dir = repo_root().join("impl/rust");
    println!("cargo:rerun-if-changed={}", impl_dir.join("buffer_vec.rs").display());
    println!("cargo:rerun-if-changed={}", impl_dir.join("buffer_helpers.rs").display());

    let pit_src = fs::read_to_string(&pit).expect("buffer.pit");
    let (_, iface) = parse_interface(&pit_src).expect("parse buffer.pit");
    let trait_tokens = rust_iface(
        &Params {
            core: syn::parse_quote!(::std),
            flags: FeatureFlags::default(),
            asyncness: None,
        },
        &iface,
    );

    let vec_impl = sanitize_rust_impl(
        &fs::read_to_string(impl_dir.join("buffer_vec.rs")).expect("buffer_vec.rs"),
    );
    let helpers_impl = sanitize_rust_impl(
        &fs::read_to_string(impl_dir.join("buffer_helpers.rs")).expect("buffer_helpers.rs"),
    );

    let fixture = format!(
        r#"{trait}

{vec_impl}

mod helpers {{
  use super::*;
{helpers}
}}

#[cfg(test)]
mod tests {{
  use super::*;

  #[test]
  fn smoke() {{
    let mut v = vec![1u8, 2, 3];
    assert_eq!(v.read8(0).unwrap(), 1);
  }}
}}
"#,
        trait = trait_tokens.to_string(),
        vec_impl = vec_impl,
        helpers = helpers_impl,
    );

    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("fixture.rs"),
        fixture,
    )
    .expect("write fixture");
}
