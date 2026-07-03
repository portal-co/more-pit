use std::fs;

use anyhow::Result;

use crate::artifact::{EmitScope, EmittedArtifact};
use crate::config::{
    GuestBundleConfig, JavaPackageAccess, OutputLayout, OutputLayoutAccess, PitRootsAccess,
    ScalaPackageAccess,
};
use crate::contributor::ContributorRegistry;
use crate::graph::{PitGraph, PitGraphEntry};
use crate::manifest::{commit_manifests, write_manifests, ManifestFragment, ManifestPipeline};
use crate::write::{dedupe_shared, rust_rel_path, write_artifacts};

#[derive(Default)]
pub struct BundleOutput {
    pub artifacts: Vec<EmittedArtifact>,
    pub manifest_fragments: Vec<ManifestFragment>,
}

pub struct GuestBundleOptions<'a, C: ?Sized> {
    pub manifest_pipeline: Option<&'a ManifestPipeline<C>>,
}

impl<'a, C: ?Sized> Default for GuestBundleOptions<'a, C> {
    fn default() -> Self {
        Self {
            manifest_pipeline: None,
        }
    }
}

/// Bulk guest generation: flat tree under `output_root`.
pub fn write_bulk_guest<C>(
    graph: &PitGraph,
    cfg: &C,
    registry: &ContributorRegistry<C>,
) -> Result<BundleOutput>
where
    C: OutputLayoutAccess
        + PitRootsAccess
        + JavaPackageAccess
        + ScalaPackageAccess
        + GuestBundleConfig,
{
    let out = cfg.output_root();
    fs::create_dir_all(out.join("rust"))?;
    fs::create_dir_all(out.join("java"))?;
    fs::create_dir_all(out.join("scala"))?;
    fs::create_dir_all(out.join("c"))?;

    let mut shared_artifacts = Vec::new();
    if let Some((_, entry)) = graph.iter().next() {
        for contributor in registry.contributors() {
            let mut emitted = contributor.emit(&entry.iface, graph, cfg)?;
            shared_artifacts.extend(emitted.drain(..).filter(|a| a.scope == EmitScope::SharedOnce));
        }
    }
    dedupe_shared(&mut shared_artifacts);
    write_artifacts(&shared_artifacts, cfg, out, None)?;

    let mut all_artifacts = shared_artifacts;

    for (_rid, entry) in graph.iter() {
        let mut per_iface = Vec::new();
        for contributor in registry.contributors() {
            let mut emitted = contributor.emit(&entry.iface, graph, cfg)?;
            emitted.retain(|a| a.scope == EmitScope::PerInterface);
            per_iface.append(&mut emitted);
        }
        let rust_rel = rust_rel_path(entry);
        write_artifacts(&per_iface, cfg, out, Some(&rust_rel))?;
        all_artifacts.extend(per_iface);
    }

    Ok(BundleOutput {
        artifacts: all_artifacts,
        manifest_fragments: Vec::new(),
    })
}

/// Self-contained guest crate for one interface (+ manifests when pipeline provided).
pub fn write_guest_bundle<C>(
    entry: &PitGraphEntry,
    graph: &PitGraph,
    cfg: &C,
    registry: &ContributorRegistry<C>,
    options: GuestBundleOptions<'_, C>,
) -> Result<BundleOutput>
where
    C: GuestBundleConfig,
{
    let out = cfg.output_root();
    if cfg.output_layout() == OutputLayout::SelfContainedCrate {
        fs::create_dir_all(out.join("src"))?;
    }
    fs::create_dir_all(out.join("java"))?;
    fs::create_dir_all(out.join("scala"))?;
    fs::create_dir_all(out.join("c"))?;

    let mut all_artifacts = Vec::new();
    for contributor in registry.contributors() {
        let mut emitted = contributor.emit(&entry.iface, graph, cfg)?;
        all_artifacts.append(&mut emitted);
    }
    dedupe_shared(&mut all_artifacts);

    let rust_rel = rust_rel_path(entry);
    write_artifacts(&all_artifacts, cfg, out, Some(&rust_rel))?;

    let mut manifest_fragments = Vec::new();
    if let Some(pipeline) = options.manifest_pipeline {
        let ctx = crate::manifest::ManifestContext {
            iface: &entry.iface,
            graph,
            artifacts: &all_artifacts,
            cfg,
            prior: &[],
        };
        manifest_fragments = pipeline.run(ctx)?;
        let committed = commit_manifests(&manifest_fragments)?;
        write_manifests(out, &committed)?;
    }

    Ok(BundleOutput {
        artifacts: all_artifacts,
        manifest_fragments,
    })
}
