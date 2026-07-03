use std::path::{Path, PathBuf};

use pit_packaging::{
    CHeaderNamingAccess, CHeaderPrefix, GuestBundleConfig, InputPathAccess, JavaPackageAccess,
    ManifestBundleConfig, OutputLayout, OutputLayoutAccess, PackageVersionAccess, PitRootsAccess,
    RuntimeAssetsAccess, ScalaPackageAccess, WasmGuestAccess,
};

/// Default guest bundle configuration (pit-cli / autobuild).
#[derive(Clone, Debug)]
pub struct DefaultGuestConfig {
    pub java_package: String,
    pub scala_package: String,
    pub tpit: bool,
    pub rust_crate_root: String,
    pub rust_salt: Vec<u8>,
    pub pit_root: PathBuf,
    pub output_root: PathBuf,
    pub layout: OutputLayout,
    pub c_header_prefix: CHeaderPrefix,
    pub teavm_runtime_dir: PathBuf,
    pub teavm_interop_version: String,
    pub package_version: String,
    pub input_path: Option<PathBuf>,
}

impl DefaultGuestConfig {
    pub fn flat_bulk(pit_root: PathBuf, output_root: PathBuf) -> Self {
        Self {
            java_package: super::DEFAULT_JAVA_PKG.into(),
            scala_package: super::DEFAULT_SCALA_PKG.into(),
            tpit: true,
            rust_crate_root: "::tpit_rt".into(),
            rust_salt: Vec::new(),
            pit_root,
            output_root,
            layout: OutputLayout::FlatGuestTree,
            c_header_prefix: CHeaderPrefix::P,
            teavm_runtime_dir: PathBuf::new(),
            teavm_interop_version: super::TEAVM_INTEROP_VER.into(),
            package_version: "0.5.0-alpha.1".into(),
            input_path: None,
        }
    }

    pub fn self_contained(
        input_path: PathBuf,
        output_root: PathBuf,
        pit_root: PathBuf,
        tpit: bool,
    ) -> Self {
        let mut cfg = Self::flat_bulk(pit_root.clone(), output_root.clone());
        cfg.layout = OutputLayout::SelfContainedCrate;
        cfg.c_header_prefix = CHeaderPrefix::R;
        cfg.tpit = tpit;
        cfg.input_path = Some(input_path);
        cfg.pit_root = pit_root;
        cfg.output_root = output_root;
        cfg
    }
}

impl OutputLayoutAccess for DefaultGuestConfig {
    fn output_layout(&self) -> OutputLayout {
        self.layout
    }
}

impl JavaPackageAccess for DefaultGuestConfig {
    fn java_package(&self) -> &str {
        &self.java_package
    }
}

impl ScalaPackageAccess for DefaultGuestConfig {
    fn scala_package(&self) -> &str {
        &self.scala_package
    }
}

impl WasmGuestAccess for DefaultGuestConfig {
    fn tpit(&self) -> bool {
        self.tpit
    }
    fn rust_crate_root(&self) -> &str {
        &self.rust_crate_root
    }
    fn rust_salt(&self) -> &[u8] {
        &self.rust_salt
    }
}

impl PitRootsAccess for DefaultGuestConfig {
    fn pit_root(&self) -> &Path {
        &self.pit_root
    }
    fn output_root(&self) -> &Path {
        &self.output_root
    }
}

impl CHeaderNamingAccess for DefaultGuestConfig {
    fn c_header_prefix(&self) -> CHeaderPrefix {
        self.c_header_prefix
    }
}

impl RuntimeAssetsAccess for DefaultGuestConfig {
    fn teavm_runtime_dir(&self) -> &Path {
        &self.teavm_runtime_dir
    }
    fn teavm_interop_version(&self) -> &str {
        &self.teavm_interop_version
    }
}

impl PackageVersionAccess for DefaultGuestConfig {
    fn package_version(&self) -> &str {
        &self.package_version
    }
}

impl InputPathAccess for DefaultGuestConfig {
    fn input_path(&self) -> Option<&Path> {
        self.input_path.as_deref()
    }
}

impl GuestBundleConfig for DefaultGuestConfig {}
impl ManifestBundleConfig for DefaultGuestConfig {}
