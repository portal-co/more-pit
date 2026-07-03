use anyhow::Result;
use pit_core::Interface;
use pit_gen::{direct_deps, rewrite_value, Backend};
use pit_lang_generic::{Java, Opts, Scala, Syntax};
use pit_packaging::{
    EmittedArtifact, JavaPackageAccess, PackagingContributor, PitGraph, ScalaPackageAccess,
};

pub struct CanonicalJavaContributor;

impl<C: JavaPackageAccess> PackagingContributor<C> for CanonicalJavaContributor {
    fn id(&self) -> &'static str {
        "canonical-java"
    }
    fn emit(&self, iface: &Interface, _graph: &PitGraph, cfg: &C) -> Result<Vec<EmittedArtifact>> {
        let rid = iface.rid_str();
        let mut opts = Opts::<Java>::default();
        for dep in direct_deps(iface) {
            let dep_hex = hex::encode(dep);
            let j = rewrite_value(Backend::Java, &dep_hex);
            if !j.is_empty() {
                opts.rewrites.insert(dep, j);
            }
        }
        Ok(vec![EmittedArtifact::new(format!("P{rid}.java"), opts.file(iface))
            .with_package(cfg.java_package())])
    }
}

pub struct CanonicalScalaContributor;

impl<C: pit_packaging::ScalaPackageAccess> PackagingContributor<C> for CanonicalScalaContributor {
    fn id(&self) -> &'static str {
        "canonical-scala"
    }
    fn emit(&self, iface: &Interface, _graph: &PitGraph, cfg: &C) -> Result<Vec<EmittedArtifact>> {
        let rid = iface.rid_str();
        let mut opts = Opts::<Scala>::default();
        for dep in direct_deps(iface) {
            let dep_hex = hex::encode(dep);
            let s = rewrite_value(Backend::Scala, &dep_hex);
            if !s.is_empty() {
                opts.rewrites.insert(dep, s);
            }
        }
        Ok(vec![EmittedArtifact::new(format!("P{rid}.scala"), opts.file(iface))
            .with_package(cfg.scala_package())])
    }
}
