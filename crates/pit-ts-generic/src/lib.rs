//! # pit-ts-generic
//!
//! > **This crate is now a thin re-export of [`pit_lang_generic`].**
//! >
//! > All types and functions live in `pit_lang_generic`; this crate exists
//! > for backwards compatibility only.  Please migrate to `pit-lang-generic`.
//!
//! ## Migration
//!
//! ```toml
//! # Before
//! pit-ts-generic = { … }
//!
//! # After
//! pit-lang-generic = { … }
//! ```
//!
//! ```ignore
//! // Before
//! use pit_ts_generic::TsOpts;
//! let opts = TsOpts { r#async: false, .. };
//!
//! // After (sync)
//! use pit_lang_generic::{Opts, TypeScript};
//! let opts = Opts::<TypeScript>::default();
//!
//! // After (async)
//! use pit_lang_generic::{Opts, TypeScriptAsync};
//! let opts = Opts::<TypeScriptAsync>::default();
//! ```
//!
//! The old `TsOpts { async: true }` mode maps to [`pit_lang_generic::TsOptsAsync`].

#![no_std]

pub use pit_lang_generic::{Opts, Syntax, TsOpts, TsOptsAsync, TypeScript, TypeScriptAsync};
