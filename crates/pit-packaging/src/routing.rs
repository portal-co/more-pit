use std::path::{Path, PathBuf};

use crate::artifact::{EmittedArtifact, Language};
use crate::config::{JavaPackageAccess, OutputLayout, OutputLayoutAccess, ScalaPackageAccess};

pub fn artifact_path<C>(
    artifact: &EmittedArtifact,
    cfg: &C,
    out: &Path,
    rust_rel: Option<&Path>,
) -> PathBuf
where
    C: OutputLayoutAccess + JavaPackageAccess + ScalaPackageAccess,
{
    let layout = cfg.output_layout();
    match artifact.language {
        Language::Java => {
            let pkg = artifact
                .package
                .as_deref()
                .unwrap_or_else(|| cfg.java_package());
            out.join("java")
                .join(pkg.replace('.', "/"))
                .join(&artifact.path)
        }
        Language::Scala => {
            let pkg = artifact
                .package
                .as_deref()
                .unwrap_or_else(|| cfg.scala_package());
            out.join("scala")
                .join(pkg.replace('.', "/"))
                .join(&artifact.path)
        }
        Language::Rust => match layout {
            OutputLayout::FlatGuestTree => {
                let rel = rust_rel.unwrap_or(Path::new("lib.rs"));
                out.join("rust").join(rel)
            }
            OutputLayout::SelfContainedCrate => out.join("src").join("lib.rs"),
        },
        Language::C => match layout {
            OutputLayout::FlatGuestTree => out.join("c").join(&artifact.path),
            OutputLayout::SelfContainedCrate => out.join(&artifact.path),
        },
        Language::Ts | Language::Other => {
            // Preserve relative paths (ts/, java/JsHandler.java at bundle root, …).
            if artifact.path.contains('/') {
                out.join(&artifact.path)
            } else {
                out.join(&artifact.path)
            }
        }
    }
}
