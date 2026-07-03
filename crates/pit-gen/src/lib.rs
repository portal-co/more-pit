//! # pit-gen (library)
//!
//! Core logic shared between the `pit-gen` binary and its compile-test suite.
//!
//! ## Public surface
//!
//! | Item | Purpose |
//! |---|---|
//! | [`Backend`] | Enum of all supported target languages |
//! | [`direct_deps`] | Extract dependency RIDs from a PIT interface |
//! | [`output_filename`] | Map (backend, hex) → generated filename |
//! | [`rewrite_value`] | Map (backend, dep_hex) → `opts.rewrites` entry |

use std::collections::BTreeMap;

use pit_core::{Arg, ArgTy, Interface, ResTy};

// ─────────────────────────────────────────────────────────────────────────────
// Backend
// ─────────────────────────────────────────────────────────────────────────────

/// All supported code-generation backends.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    C,
    Go,
    Haxe,
    Ts,
    TsAsync,
    Swift,
    Haskell,
    Rust,
    Java,
    Scala,
    ScalaC,
    JsTeavm,
}

impl Backend {
    /// Parse a backend name string (as used on the CLI).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "c" => Some(Self::C),
            "go" => Some(Self::Go),
            "haxe" => Some(Self::Haxe),
            "ts" => Some(Self::Ts),
            "ts-async" => Some(Self::TsAsync),
            "swift" => Some(Self::Swift),
            "haskell" => Some(Self::Haskell),
            "rust" => Some(Self::Rust),
            "java" => Some(Self::Java),
            "scala" => Some(Self::Scala),
            "scala-c" => Some(Self::ScalaC),
            "js-teavm" => Some(Self::JsTeavm),
            _ => None,
        }
    }

    /// Canonical CLI name for this backend.
    pub fn name(self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Go => "go",
            Self::Haxe => "haxe",
            Self::Ts => "ts",
            Self::TsAsync => "ts-async",
            Self::Swift => "swift",
            Self::Haskell => "haskell",
            Self::Rust => "rust",
            Self::Java => "java",
            Self::Scala => "scala",
            Self::ScalaC => "scala-c",
            Self::JsTeavm => "js-teavm",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Dependency extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Collect the set of direct dependency RIDs for an interface
/// (all `ResTy::Of(id)` values that differ from the interface's own RID).
pub fn direct_deps(iface: &Interface) -> Vec<[u8; 32]> {
    let this = iface.rid();
    let mut seen: BTreeMap<[u8; 32], ()> = BTreeMap::new();
    for sig in iface.methods.values() {
        for arg in sig.params.iter().chain(sig.rets.iter()) {
            if let ArgTy::Resource { ty: ResTy::Of(id), .. } = &arg.ty {
                if *id != this {
                    seen.insert(*id, ());
                }
            }
        }
    }
    seen.into_keys().collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// File naming
// ─────────────────────────────────────────────────────────────────────────────

/// Return the output filename for an interface given a backend and its hex RID.
pub fn output_filename(backend: Backend, hex: &str) -> String {
    match backend {
        Backend::C => format!("P{hex}.h"),
        Backend::Go => format!("P{hex}.go"),
        Backend::Haxe => format!("P{hex}.hx"),
        Backend::Ts => format!("P{hex}.ts"),
        Backend::TsAsync => format!("AP{hex}.ts"),
        Backend::Swift => format!("P{hex}.swift"),
        Backend::Haskell => format!("P{hex}.hs"),
        Backend::Rust => format!("p_{hex}.rs"),
        Backend::Java => format!("P{hex}.java"),
        Backend::Scala => format!("P{hex}.scala"),
        Backend::ScalaC => format!("P{hex}Native.scala"),
        Backend::JsTeavm => format!("P{hex}_js.ts"),
    }
}

/// Return the value stored in `opts.rewrites[dep_rid]` for a given backend.
///
/// This is what each backend's `render_file` uses to generate
/// import / include statements.  An empty string means "no import needed"
/// (e.g. same-package backends like Go and Haxe in single-package mode).
pub fn rewrite_value(backend: Backend, dep_hex: &str) -> String {
    match backend {
        Backend::C => format!("P{dep_hex}.h"),
        Backend::Go => String::new(),      // same package — no import
        Backend::Haxe => String::new(),    // same package — no import
        Backend::Ts => format!("./P{dep_hex}"),
        Backend::TsAsync => format!("./AP{dep_hex}"),
        Backend::Swift => String::new(),   // all files compiled together
        Backend::Haskell => format!("P{dep_hex}"),
        Backend::Rust => String::new(),    // all traits in one file
        Backend::Java => format!("pc.portal.pit.guest.P{dep_hex}"),
        Backend::Scala => format!("pc.portal.pit.guest.scala.P{dep_hex}"),
        Backend::ScalaC => format!("pc.portal.pit.guest.scala.P{dep_hex}"),
        Backend::JsTeavm => format!("./P{dep_hex}"),
    }
}
