---
name: pit-integrations
description: Add, debug, or review a `more-pit` code-generation backend, generated PIT bindings, `Syntax`/`Opts`, `pit-gen`, or SDK/PIT conversion shims. Read this before changing backend architecture or claiming a language is supported.
---

# PIT integrations

PIT is intentionally not a host-language type system. Each backend maps the same small interface-and-integer contract into a target language, while resource transport and runtime policy remain integration concerns. This separation is why a backend must state its actual support surface instead of pretending that a PIT binding alone provides a working cross-language adapter.

## Work sequence

1. Read [`AGENTS.md`](../../../AGENTS.md), especially the architecture and hard-fail compile-test rules.
2. Locate the real implementation. `pit-lang-generic` owns the shared `Syntax` backends; `pit-rust-generic` has its own Rust pipeline; compatibility crates re-export these implementations.
3. Check the target backend's current type mapping and known limitations in [`references/backends.md`](references/backends.md), then verify it against source.
4. For a new language feature, implement the whole claimed surface, add semantic tests, and add a real compiler test. Missing toolchains are environment failures, not skips.
5. Keep transport and runtime dependencies explicit. A generated signature or conversion contract is not itself a transport implementation.

## Architecture boundaries

- `pit-lang-generic::Syntax` renders PIT types, methods, interfaces, and complete files for its target languages.
- `pit-rust-generic` generates Rust traits independently, using `proc_macro2`, `quote`, and `syn`.
- `pit-gen` selects PIT input files and dispatches generators. It does not make a language backend shim-capable merely by listing that backend.
- `pit-lang-generic::wire` owns abstract wire-field layout. TeaVM/WASM glue stays in `pit`/`pit-js-teavm`; Scala native FFI stays in `pit-scala-c-bridge`.
- `pit-sdk-bridge` owns SDK-only schema lowering and paired value converters; `pit-sdk-gen --backend rust|ts` selects those shims. Resource resolution is supplied by the host. This is separate from `pit-gen`, which generates PIT-only bindings.

## Current backend inventory

Use checked-in `pit-gen` dispatch and crate source as the source of truth. The shared syntax pipeline includes C, Go, Haxe, TypeScript (sync and async), Swift, Haskell, Java, and Scala; Rust uses `pit-rust-generic`. `pit-gen` also has dedicated Scala-C and JS-TeaVM paths. Availability, compile coverage, ownership behavior, nullable handling, and conversion-shim support differ by backend.

For backend-specific mappings, caveats, and the explicitly supported shim subset, read [`references/backends.md`](references/backends.md). The contract is in the sibling `portal-solutions-sdk/docs/sdk-to-pit-spec.md`; implementation tests are under `crates/pit-sdk-bridge`. Compile-test requirements are in [`../../../docs/compile-tests.md`](../../../docs/compile-tests.md). For PIT syntax and RID rules, read [`../pit-language/SKILL.md`](../pit-language/SKILL.md).
