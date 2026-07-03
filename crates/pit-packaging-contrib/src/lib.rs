//! Default packaging contributors, configuration, manifest layers, and merge strategies.

mod canonical;
mod js_teavm;
mod scala_native;
pub mod config;
pub mod manifest;
pub mod merge;

pub use canonical::{CanonicalJavaContributor, CanonicalScalaContributor};
pub use js_teavm::JsTeavmContributor;
pub use scala_native::ScalaNativeContributor;
pub use config::DefaultGuestConfig;
pub use manifest::{
    default_guest_manifest_pipeline, BazelMirrorLayer, CargoTomlLayer, CmakeCargoBridgeLayer,
    MavenLayer, SbtLayer,
};
pub use merge::{AppendToPath, CargoMetadataOverlay, ReplaceFile};

pub const DEFAULT_JAVA_PKG: &str = "pc.portal.pit.guest";
pub const DEFAULT_SCALA_PKG: &str = "pc.portal.pit.guest.scala";
pub const TEAVM_INTEROP_VER: &str = "0.10.2";
