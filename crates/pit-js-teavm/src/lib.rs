//! # pit-js-teavm
//!
//! Bidirectional object-to-object bridge between Java/Scala TeaVM (JS backend)
//! and TypeScript `P{rid}` implementations — without WASM wire layouts.

mod guest;
mod java;
mod scala;
mod ts;

use pit_core::Interface;
pub use pit_lang_generic::wire::EmittedFile;

pub const DEFAULT_JAVA_PKG: &str = "pc.portal.pit.guest";
pub const DEFAULT_SCALA_PKG: &str = "pc.portal.pit.guest.scala";

/// Code-generation context for pit-js-teavm emitters.
pub struct JsTeavmContext<'a> {
    pub java_pkg: &'a str,
    pub scala_pkg: &'a str,
}

impl Default for JsTeavmContext<'_> {
    fn default() -> Self {
        Self {
            java_pkg: DEFAULT_JAVA_PKG,
            scala_pkg: DEFAULT_SCALA_PKG,
        }
    }
}

/// Emitter for TeaVM JS object bridge glue.
#[derive(Default, Clone, Copy, Debug)]
pub struct JsTeavmBackend;

impl JsTeavmBackend {
    /// Shared runtime files emitted once per generation run.
    pub fn emit_shared_files(ctx: &JsTeavmContext<'_>) -> Vec<EmittedFile> {
        vec![
            EmittedFile {
                path: "ts/pit-js-runtime.ts".to_string(),
                content: ts::emit_runtime(),
            },
            EmittedFile {
                path: format!("java/JsHandler.java"),
                content: java::emit_handler(ctx.java_pkg),
            },
            EmittedFile {
                path: format!("java/JsRuntime.java"),
                content: java::emit_runtime(ctx.java_pkg),
            },
            EmittedFile {
                path: format!("scala/JsHandler.scala"),
                content: scala::emit_handler(ctx.scala_pkg),
            },
            EmittedFile {
                path: format!("scala/JsRuntime.scala"),
                content: scala::emit_runtime(ctx.scala_pkg),
            },
        ]
    }

    /// Per-interface adapter files (TS + Java + Scala).
    pub fn emit_interface_files(iface: &Interface, ctx: &JsTeavmContext<'_>) -> Vec<EmittedFile> {
        let rid = iface.rid_str();
        vec![
            EmittedFile {
                path: format!("ts/P{rid}_js.ts"),
                content: ts::emit_interface_js(iface),
            },
            EmittedFile {
                path: format!("java/P{rid}Js.java"),
                content: java::emit_interface_js(iface, ctx),
            },
            EmittedFile {
                path: format!("scala/P{rid}Js.scala"),
                content: scala::emit_interface_js(iface, ctx),
            },
        ]
    }

    /// All files for one interface (includes shared runtimes — caller should dedupe shared).
    pub fn emit_files(iface: &Interface, ctx: &JsTeavmContext<'_>) -> Vec<EmittedFile> {
        let mut files = Self::emit_shared_files(ctx);
        files.extend(Self::emit_interface_files(iface, ctx));
        files
    }
}

#[cfg(test)]
mod tests {
    use pit_core::parse_interface;

    use super::*;

    const BUFFER: &str = r#"{
        read8(I32) -> (I32);
        write8(I32,I32) -> ();
        size() -> (I32)
    }"#;

    #[test]
    fn emits_js_export_and_proxy() {
        let (_, iface) = parse_interface(BUFFER).unwrap();
        let files = JsTeavmBackend::emit_interface_files(&iface, &JsTeavmContext::default());
        let java = files
            .iter()
            .find(|f| f.path.ends_with("Js.java"))
            .expect("java glue");
        assert!(java.content.contains("ExportObject"));
        assert!(java.content.contains("JsProxy"));
        assert!(java.content.contains("@JSExport"));
        assert!(!java.content.contains("tpit/"));
        assert!(!java.content.contains("WireScalar"));
    }

    #[test]
    fn ts_glue_has_to_from_js() {
        let (_, iface) = parse_interface(BUFFER).unwrap();
        let files = JsTeavmBackend::emit_interface_files(&iface, &JsTeavmContext::default());
        let ts = files.iter().find(|f| f.path.ends_with("_js.ts")).expect("ts glue");
        assert!(ts.content.contains("export function toJs"));
        assert!(ts.content.contains("export function fromJs"));
    }
}
