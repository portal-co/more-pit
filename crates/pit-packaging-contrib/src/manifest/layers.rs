use anyhow::Result;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use pit_core::Interface;
use pit_packaging::{
    EmittedArtifact, JavaPackageAccess, Language, ManifestBundleConfig, ManifestContext,
    ManifestFragment, ManifestLayer, ManifestPipeline, PackageVersionAccess, RuntimeAssetsAccess,
    WasmGuestAccess,
};

use crate::merge::{CargoMetadataOverlay, ReplaceFile};

fn java_glob_for_rid<C: JavaPackageAccess>(ctx: &ManifestContext<'_, C>, rid: &str) -> String {
    let pkg_path = ctx.cfg.java_package().replace('.', "/");
    format!("java/{pkg_path}/P{rid}*.java")
}

fn java_artifacts_for_rid<'a>(
    artifacts: &'a [EmittedArtifact],
    rid: &str,
) -> Vec<&'a EmittedArtifact> {
    artifacts
        .iter()
        .filter(|a| a.language == Language::Java && a.path.contains(rid))
        .collect()
}

pub struct CargoTomlLayer;

impl<C> ManifestLayer<C> for CargoTomlLayer
where
    C: ManifestBundleConfig + WasmGuestAccess + PackageVersionAccess,
{
    fn id(&self) -> &'static str {
        "cargo-toml"
    }
    fn emit(&self, ctx: &ManifestContext<'_, C>) -> Result<Vec<ManifestFragment>> {
        let rid = ctx.iface.rid_str();
        let crate_name = format!(
            "pit-autogen-{}-{}",
            BASE64_URL_SAFE_NO_PAD.encode(ctx.iface.rid()),
            if ctx.cfg.tpit() { "tpit" } else { "externref" }
        );
        let dep = if ctx.cfg.tpit() {
            format!(
                "tpit-rt = {{ version = \"{}\" }}",
                ctx.cfg.package_version()
            )
        } else {
            "externref = \"0.2.0\"".to_owned()
        };
        let content = format!(
            r#"[package]
name = "{crate_name}"
version = "{}"
edition = "2021"
license = "CC0-1.0"
description = "Automatically generated"
[dependencies]
{dep}
"#,
            ctx.cfg.package_version()
        );
        let _ = rid;
        Ok(vec![ManifestFragment {
            path: "Cargo.toml".into(),
            content,
            merge: Box::new(ReplaceFile),
        }])
    }
}

pub struct BazelMirrorLayer;

impl<C> ManifestLayer<C> for BazelMirrorLayer
where
    C: ManifestBundleConfig + JavaPackageAccess + WasmGuestAccess + PackageVersionAccess,
{
    fn id(&self) -> &'static str {
        "bazel-mirror"
    }
    fn emit(&self, ctx: &ManifestContext<'_, C>) -> Result<Vec<ManifestFragment>> {
        let rid = ctx.iface.rid_str();
        let prefix = if ctx.cfg.c_header_prefix() == pit_packaging::CHeaderPrefix::R {
            "R"
        } else {
            "P"
        };
        let bazel_cc = format!(
            r#"cc_library(
    name = "r{rid}",
    srcs = ["{prefix}{rid}.c"],
    hdrs = ["{prefix}{rid}.h"],
    visibility = ["//visibility:public"],
    deps = ["@wasm_handler"]
)"#
        );
        let crate_name = format!(
            "pit-autogen-{}-{}",
            BASE64_URL_SAFE_NO_PAD.encode(ctx.iface.rid()),
            if ctx.cfg.tpit() { "tpit" } else { "externref" }
        );
        let rust_dep = if ctx.cfg.tpit() { "tpit-rt" } else { "externref" };
        let java_glob = java_glob_for_rid(ctx, &rid);
        let scala_pkg = ctx.cfg.scala_package().replace('.', "/");
        let build = format!(
            r#"package(default_visibility = ["//visibility:public"])
load("@rules_rust//rust:defs.bzl", "rust_library")
load("@io_bazel_rules_scala//scala:scala.bzl", "scala_library")
load("@rules_java//java:defs.bzl", "java_library")
{bazel_cc}
rust_library(
    name = "{crate_name}",
    srcs = ["src/lib.rs"],
    deps = ["@crates//:{rust_dep}"]
)
java_library(
    name = "teavm-java-{rid}",
    srcs = glob(["{java_glob}"]),
    deps = ["@org-teavm-teavm-interop"]
)
scala_library(
    name = "teavm-scala-{rid}",
    srcs = glob(["scala/{scala_pkg}/P{rid}*.scala"]),
    deps = ["@org-teavm-teavm-interop"]
)
"#
        );
        Ok(vec![
            ManifestFragment {
                path: "BUILD.bazel".into(),
                content: build,
                merge: Box::new(ReplaceFile),
            },
            ManifestFragment {
                path: "Cargo.toml".into(),
                content: format!(
                    "additive_build_file_content = \"\"\"\n{bazel_cc}\n\"\"\"\nextra_aliased_targets = {{ r{rid} = \"r{rid}\" }}\n"
                ),
                merge: Box::new(CargoMetadataOverlay {
                    section: "bazel".into(),
                }),
            },
        ])
    }
}

pub struct CmakeCargoBridgeLayer;

impl<C> ManifestLayer<C> for CmakeCargoBridgeLayer
where
    C: ManifestBundleConfig,
{
    fn id(&self) -> &'static str {
        "cmake-cargo-bridge"
    }
    fn emit(&self, ctx: &ManifestContext<'_, C>) -> Result<Vec<ManifestFragment>> {
        let rid = ctx.iface.rid_str();
        let prefix = if ctx.cfg.c_header_prefix() == pit_packaging::CHeaderPrefix::R {
            "R"
        } else {
            "P"
        };
        Ok(vec![ManifestFragment {
            path: "CMakeLists.txt".into(),
            content: format!(
                r#"add_library(r{rid} STATIC {prefix}{rid}.c)
target_include_directories(r{rid} PUBLIC ${{CMAKE_CURRENT_SOURCE_DIR}})
target_link_libraries(r{rid} PUBLIC wasm_handler)
"#
            ),
            merge: Box::new(ReplaceFile),
        }])
    }
}

pub struct MavenLayer;

impl<C> ManifestLayer<C> for MavenLayer
where
    C: ManifestBundleConfig + JavaPackageAccess + RuntimeAssetsAccess,
{
    fn id(&self) -> &'static str {
        "maven"
    }
    fn emit(&self, ctx: &ManifestContext<'_, C>) -> Result<Vec<ManifestFragment>> {
        let rid = ctx.iface.rid_str();
        let java_files = java_artifacts_for_rid(ctx.artifacts, &rid);
        let _ = java_files;
        Ok(vec![ManifestFragment {
            path: "pom.xml".into(),
            content: format!(
                r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>pc.portal.pit</groupId>
  <artifactId>pit-autogen-{rid}</artifactId>
  <version>{}</version>
  <dependencies>
    <dependency>
      <groupId>org.teavm</groupId>
      <artifactId>teavm-interop</artifactId>
      <version>{}</version>
    </dependency>
  </dependencies>
</project>"#,
                ctx.cfg.package_version(),
                ctx.cfg.teavm_interop_version()
            ),
            merge: Box::new(ReplaceFile),
        }])
    }
}

pub struct SbtLayer;

impl<C> ManifestLayer<C> for SbtLayer
where
    C: ManifestBundleConfig + RuntimeAssetsAccess,
{
    fn id(&self) -> &'static str {
        "sbt"
    }
    fn emit(&self, ctx: &ManifestContext<'_, C>) -> Result<Vec<ManifestFragment>> {
        Ok(vec![ManifestFragment {
            path: "build.sbt".into(),
            content: format!(
                r#"ThisBuild / scalaVersion := "3.3.3"
libraryDependencies += "org.teavm" % "teavm-interop" % "{}"
"#,
                ctx.cfg.teavm_interop_version()
            ),
            merge: Box::new(ReplaceFile),
        }])
    }
}

pub fn default_guest_manifest_pipeline<C: ManifestBundleConfig + JavaPackageAccess + WasmGuestAccess + RuntimeAssetsAccess + PackageVersionAccess>(
) -> ManifestPipeline<C> {
    ManifestPipeline::new()
        .nest(CargoTomlLayer)
        .nest(BazelMirrorLayer)
        .nest(CmakeCargoBridgeLayer)
        .nest(MavenLayer)
        .nest(SbtLayer)
}
