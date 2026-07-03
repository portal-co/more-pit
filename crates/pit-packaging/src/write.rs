use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::artifact::{EmitScope, EmittedArtifact};
use crate::config::{JavaPackageAccess, OutputLayoutAccess, PitRootsAccess, ScalaPackageAccess};
use crate::graph::PitGraphEntry;
use crate::routing::artifact_path;

pub fn write_artifacts<C>(
    artifacts: &[EmittedArtifact],
    cfg: &C,
    out: &Path,
    rust_rel: Option<&Path>,
) -> Result<()>
where
    C: OutputLayoutAccess + JavaPackageAccess + ScalaPackageAccess,
{
    for artifact in artifacts {
        let path = artifact_path(artifact, cfg, out, rust_rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &artifact.content)?;
    }
    Ok(())
}

/// Merge shared-once artifacts by path (last wins).
pub fn dedupe_shared(artifacts: &mut Vec<EmittedArtifact>) {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::new();
    for artifact in artifacts.drain(..) {
        if artifact.scope == EmitScope::SharedOnce {
            if let Some(idx) = seen.get(&artifact.path) {
                out[*idx] = artifact;
            } else {
                seen.insert(artifact.path.clone(), out.len());
                out.push(artifact);
            }
        } else {
            out.push(artifact);
        }
    }
    *artifacts = out;
}

pub fn rust_rel_path(entry: &PitGraphEntry) -> std::path::PathBuf {
    entry.rel_path.with_extension("rs")
}
