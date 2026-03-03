//! # pit-c-generic
//!
//! > **This crate is now a thin re-export of [`pit_lang_generic::c`].**
//! >
//! > All types, macros, and `Display` implementations live in
//! > `pit_lang_generic::c`; this crate exists for backwards compatibility only.
//! > Please migrate to `pit-lang-generic`.
//!
//! ## Migration
//!
//! ```toml
//! # Before
//! pit-c-generic = { … }
//!
//! # After
//! pit-lang-generic = { … }
//! ```
//!
//! ```ignore
//! // Before
//! use pit_c_generic::{C, PureC};
//!
//! // After
//! use pit_lang_generic::c::{C, PureC};
//! ```

#![no_std]

/// Re-export the `c_disp!` macro at the crate root so existing
/// `pit_c_generic::c_disp!(…)` invocations continue to work.
pub use pit_lang_generic::c_disp;

pub use pit_lang_generic::c::{C, PureC, __};
