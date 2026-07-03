use std::path::{Path, PathBuf};

use pit_packaging::{artifact_path, EmittedArtifact, OutputLayout};
use pit_packaging_contrib::DefaultGuestConfig;

#[test]
fn java_artifacts_route_to_package_directory() {
    let mut cfg = DefaultGuestConfig::flat_bulk(PathBuf::from("/pit"), PathBuf::from("/out"));
    cfg.java_package = "com.example.guest".into();
    let artifact = EmittedArtifact::new("PabcTeaVm.java", "// stub")
        .with_package("com.example.guest");
    let path = artifact_path(&artifact, &cfg, Path::new("/out"), None);
    assert_eq!(
        path,
        PathBuf::from("/out/java/com/example/guest/PabcTeaVm.java")
    );
}

#[test]
fn package_mode_c_headers_land_at_bundle_root() {
    let mut cfg = DefaultGuestConfig::flat_bulk(PathBuf::from("/pit"), PathBuf::from("/out"));
    cfg.layout = OutputLayout::SelfContainedCrate;
    let artifact = EmittedArtifact::new("Rabc.h", "// stub");
    let path = artifact_path(&artifact, &cfg, Path::new("/out"), None);
    assert_eq!(path, PathBuf::from("/out/Rabc.h"));
}
