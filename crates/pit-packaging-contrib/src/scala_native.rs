use anyhow::Result;
use pit_core::Interface;
use pit_lang_generic::wire::FfiBackend;
use pit_packaging::{
    EmittedArtifact, Language, PackagingContributor, PitGraph, ScalaPackageAccess,
};
use pit_scala_c_bridge::{ScalaNativeBackend, ScalaNativeContext};

pub struct ScalaNativeContributor {
    pub c_prefix: String,
}

impl Default for ScalaNativeContributor {
    fn default() -> Self {
        Self {
            c_prefix: "P".into(),
        }
    }
}

impl<C> PackagingContributor<C> for ScalaNativeContributor
where
    C: ScalaPackageAccess,
{
    fn id(&self) -> &'static str {
        "scala-native"
    }

    fn emit(&self, iface: &Interface, _graph: &PitGraph, cfg: &C) -> Result<Vec<EmittedArtifact>> {
        let ctx = ScalaNativeContext {
            scala_pkg: cfg.scala_package(),
            c_prefix: &self.c_prefix,
        };
        Ok(ScalaNativeBackend
            .emit_files(iface, &ctx)
            .into_iter()
            .map(|f| {
                let is_scala = f.path.ends_with(".scala");
                let mut a = EmittedArtifact::new(f.path, f.content);
                if is_scala {
                    a = a.with_package(cfg.scala_package()).with_language(Language::Scala);
                } else {
                    a = a.with_language(Language::C);
                }
                a
            })
            .collect())
    }
}
