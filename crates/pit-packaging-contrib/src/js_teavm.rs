use anyhow::Result;
use pit_core::Interface;
use pit_js_teavm::{JsTeavmBackend, JsTeavmContext};
use pit_packaging::{
    EmittedArtifact, JavaPackageAccess, Language, PackagingContributor, PitGraph, ScalaPackageAccess,
};

fn artifact_from_file(
    path: &str,
    content: String,
    java_pkg: &str,
    scala_pkg: &str,
    shared: bool,
) -> EmittedArtifact {
    let (rel, package) = if let Some(stripped) = path.strip_prefix("java/") {
        (stripped.to_string(), Some(java_pkg.to_string()))
    } else if let Some(stripped) = path.strip_prefix("scala/") {
        (stripped.to_string(), Some(scala_pkg.to_string()))
    } else {
        (path.to_string(), None)
    };
    let mut a = EmittedArtifact::new(rel, content);
    if path.ends_with(".java") {
        a = a.with_language(Language::Java);
    } else if path.ends_with(".scala") {
        a = a.with_language(Language::Scala);
    } else if path.ends_with(".ts") {
        a = a.with_language(Language::Ts);
    }
    if let Some(pkg) = package {
        a = a.with_package(pkg);
    }
    if shared {
        a = a.shared();
    }
    a
}

pub struct JsTeavmContributor;

impl<C> PackagingContributor<C> for JsTeavmContributor
where
    C: JavaPackageAccess + ScalaPackageAccess,
{
    fn id(&self) -> &'static str {
        "js-teavm"
    }

    fn emit(&self, iface: &Interface, _graph: &PitGraph, cfg: &C) -> Result<Vec<EmittedArtifact>> {
        let ctx = JsTeavmContext {
            java_pkg: cfg.java_package(),
            scala_pkg: cfg.scala_package(),
        };
        let mut out = JsTeavmBackend::emit_shared_files(&ctx)
            .into_iter()
            .map(|f| {
                artifact_from_file(
                    &f.path,
                    f.content,
                    cfg.java_package(),
                    cfg.scala_package(),
                    true,
                )
            })
            .collect::<Vec<_>>();
        out.extend(
            JsTeavmBackend::emit_interface_files(iface, &ctx)
                .into_iter()
                .map(|f| {
                    artifact_from_file(
                        &f.path,
                        f.content,
                        cfg.java_package(),
                        cfg.scala_package(),
                        false,
                    )
                }),
        );
        Ok(out)
    }
}
