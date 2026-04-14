//! # pit-haskell-generic
//!
//! > **This crate is a thin re-export of [`pit_lang_generic`].**
//! >
//! > All types live in `pit_lang_generic`; this crate exists for
//! > discoverability and as a standalone dependency for consumers that only
//! > need the Haskell backend.
//!
//! ## Overview
//!
//! Generates monad-generic Haskell typeclasses from PIT interface definitions.
//! Each PIT interface becomes a typeclass parameterised over:
//!
//! - `self` — the implementor / resource type
//! - `m`    — any type constructor satisfying `Monad m`
//!
//! Every method takes `self` as its first argument and wraps the return
//! value(s) in `m (...)`, making the interface usable with `IO`, `State`,
//! `Reader`, `STM`, or any other monad.
//!
//! ## Quick start
//!
//! ```ignore
//! use pit_haskell_generic::{HaskellOpts, Haskell, Opts};
//! use pit_core::Interface;
//!
//! let iface: Interface = /* parse from PIT format */;
//!
//! // Zero configuration needed — just use Default
//! let code = HaskellOpts::default().interface(&iface);
//! println!("{code}");
//! ```
//!
//! ## Generated code shape
//!
//! ```haskell
//! class Monad m => P<hex_id> self m where
//!   p<hex_id>_method :: self -> Data.Word.Word32 -> m (Data.Word.Word64)
//! ```
//!
//! ## Migration from `pit-lang-generic`
//!
//! ```toml
//! # Either works — they re-export identical types
//! pit-haskell-generic = { … }
//! pit-lang-generic    = { … }
//! ```
//!
//! ```ignore
//! // pit-lang-generic
//! use pit_lang_generic::{Opts, Haskell, HaskellOpts};
//!
//! // pit-haskell-generic (identical)
//! use pit_haskell_generic::{Opts, Haskell, HaskellOpts};
//! ```

#![no_std]

pub use pit_lang_generic::{Haskell, HaskellOpts, Opts, Syntax};
