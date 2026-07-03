use std::path::{Path, PathBuf};

/// Flat tree under `generated/guest/` vs self-contained per-interface crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OutputLayout {
    #[default]
    FlatGuestTree,
    SelfContainedCrate,
}

pub trait OutputLayoutAccess {
    fn output_layout(&self) -> OutputLayout;
}

pub trait JavaPackageAccess {
    fn java_package(&self) -> &str;
}

pub trait ScalaPackageAccess {
    fn scala_package(&self) -> &str;
}

pub trait WasmGuestAccess {
    fn tpit(&self) -> bool;
    /// Root path token for generated Rust guest crates (e.g. `::tpit_rt`).
    fn rust_crate_root(&self) -> &str;
    fn rust_salt(&self) -> &[u8] {
        &[]
    }
}

pub trait PitRootsAccess {
    fn pit_root(&self) -> &Path;
    fn output_root(&self) -> &Path;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CHeaderPrefix {
    #[default]
    P,
    /// Legacy package-mode `R{rid}.h` naming.
    R,
}

pub trait CHeaderNamingAccess {
    fn c_header_prefix(&self) -> CHeaderPrefix;
}

pub trait RuntimeAssetsAccess {
    fn teavm_runtime_dir(&self) -> &Path;
    fn teavm_interop_version(&self) -> &str;
}

/// Convenience supertrait for full guest bundle callers.
pub trait GuestBundleConfig:
    OutputLayoutAccess
    + JavaPackageAccess
    + ScalaPackageAccess
    + WasmGuestAccess
    + PitRootsAccess
    + CHeaderNamingAccess
    + RuntimeAssetsAccess
{
}

/// Crate version string injected into generated manifests.
pub trait PackageVersionAccess {
    fn package_version(&self) -> &str;
}

impl<T: PackageVersionAccess> PackageVersionAccess for &T {
    fn package_version(&self) -> &str {
        (**self).package_version()
    }
}

/// Optional pit file path for package-mode (single interface input).
pub trait InputPathAccess {
    fn input_path(&self) -> Option<&Path>;
}

impl<T: InputPathAccess> InputPathAccess for &T {
    fn input_path(&self) -> Option<&Path> {
        (**self).input_path()
    }
}

/// Combined bounds used by WASM manifest layers in contrib.
pub trait ManifestBundleConfig:
    GuestBundleConfig + PackageVersionAccess + InputPathAccess + CHeaderNamingAccess + Clone
{
}

/// Helper for building output paths under layout roots.
pub fn layout_subdir(out: &Path, layout: OutputLayout, name: &str) -> PathBuf {
    match layout {
        OutputLayout::FlatGuestTree => out.join(name),
        OutputLayout::SelfContainedCrate => out.join(name),
    }
}
