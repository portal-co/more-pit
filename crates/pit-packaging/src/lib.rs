//! Extensible PIT packaging orchestration.
//!
//! Core is generic over configuration accessor traits; contributors and manifest
//! layers declare the bounds they need. See [`config`] and [`contributor`].

mod artifact;
mod bundle;
mod config;
mod contributor;
mod graph;
mod manifest;
mod routing;
mod write;

pub use artifact::{EmitScope, EmittedArtifact, Language};
pub use bundle::{write_bulk_guest, write_guest_bundle, BundleOutput, GuestBundleOptions};
pub use config::{
    CHeaderNamingAccess, CHeaderPrefix, GuestBundleConfig, InputPathAccess, JavaPackageAccess,
    ManifestBundleConfig, OutputLayout, OutputLayoutAccess, PackageVersionAccess, PitRootsAccess,
    RuntimeAssetsAccess, ScalaPackageAccess, WasmGuestAccess,
};
pub use contributor::{PackagingContributor, ContributorRegistry};
pub use graph::{
    collect_pit_graph, find_pit_root, single_iface_graph, PitGraph, PitGraphEntry,
};
pub use manifest::{
    commit_manifests, ManifestContext, ManifestFragment, ManifestLayer, ManifestMerge,
    ManifestPipeline, NestableManifestLayer,
};
pub use routing::artifact_path;
pub use write::write_artifacts;
