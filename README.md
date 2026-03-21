# more-pit

Code generation libraries for [PIT (Portal Interface Types)](https://github.com/portal-co/pit-core) interfaces.

This is a Cargo workspace of Rust crates that take parsed PIT interface definitions and emit source code for various target languages. It depends on `pit-core` for parsing and the core data structures (`Interface`, `Sig`, `Arg`, etc.).

Requires Rust **nightly** (see `rust-toolchain.toml`).

## Status

Early-stage / experimental. The workspace compiles and the code generators produce output, but there are no tests, no published crates, and several backends contain `todo!()` branches for uncommon type combinations. The `pit-to-capnp` and `pit-wit-bridge` crates define traits but contain no actual conversion logic — they are stubs.

## Workspace layout

```
crates/
  pit-lang-generic/   # Core multi-language backend (C, Go, Haxe, TypeScript, Swift)
  pit-rust-generic/   # Rust trait generator (proc-macro2 / quote)
  pit-c-generic/      # Backwards-compat re-export of pit-lang-generic::c
  pit-go-generic/     # Backwards-compat re-export of pit-lang-generic
  pit-haxe-generic/   # Backwards-compat re-export of pit-lang-generic
  pit-swift-generic/  # Backwards-compat re-export of pit-lang-generic
  pit-ts-generic/     # Backwards-compat re-export of pit-lang-generic
  pit-to-capnp/       # Stub: Capnp trait + ViaCapnp Display wrapper
  pit-wit-bridge/     # Stub: ToWIT trait + ViaWIT Display wrapper
pit/
  common/             # Example .pit interface definition files
```

## How it works

### PIT interface IDs

Every PIT interface has a 32-byte ID derived from its canonical representation (SHA3-256 hash). All generated type/trait names embed this ID as a hex string, e.g. `P<64-char-hex>`. This makes names stable and unique across languages without a central registry.

### pit-lang-generic

The central crate. Defines a `Syntax` trait with three methods:

- `render_ty` — converts a PIT `Arg` to a language type
- `render_meth` — converts a PIT `Sig` to a method declaration
- `render_interface` — converts a full `Interface` to a type/interface declaration

All backends share a single `Opts<S: Syntax>` struct. `S` is zero-sized for most backends; the C backend uses it to carry an optional type-name prefix string.

The crate is `no_std` (uses `extern crate alloc`). Rendering is `Display`-based to avoid unnecessary heap allocations; the C backend in particular goes through a `Display`-only pipeline.

Implemented backends:

| Syntax type | Generated form | Notes |
|---|---|---|
| `C` | `#define <prefix><hex>_t_IFACE_<method>(CUR,METH) vfunc(...)` + `interface(...)` | Uses `vfunc` macro pattern for vtables |
| `Go` | `type P<hex> interface { P<hex>_<method>(p0 uint32) (uint64) }` | `I64` maps to `uint64`, `I32` to `uint32` |
| `Haxe` | `interface P<hex> { P<hex>_<method>(p0: haxe.Int32): {r0: Float} }` | `I64` maps to `haxe.Int32` (same as I32 — likely a known limitation) |
| `TypeScript` | `export type P<hex> = { P<hex>_<method>(p0: number): [number] }` | |
| `TypeScriptAsync` | Same but returns `[T] \| Promise<[T]>`; type names prefixed `AP<hex>` | |
| `Swift` | `open protocol P<hex> { open P<hex>_<method>(p0 _: UInt32) -> (r0 _: UInt64) }` | |

Package rewrites (for Go and Haxe cross-package resource references) are configured via `Opts::rewrites: BTreeMap<[u8; 32], String>`.

### pit-rust-generic

Generates Rust trait definitions using `proc-macro2` and `quote`. The main entry point is `interface(&Params, &Interface) -> TokenStream`.

Generated output per interface:
- A `struct P<hex>Error {}` that implements `std::error::Error`
- A `trait P<hex><'bound>` with an associated `type Error` and one method per PIT method
- Optionally (with `FeatureFlags::specialization`), a `default impl` block that returns `Err(P<hex>Error{})` for all methods

Configurable via `Params`:
- `core: syn::Path` — path prefix for standard types (`::core` or `::std`)
- `asyncness: Option<syn::token::Async>` — generate async trait methods
- `flags.specialization: bool` — emit `default impl` (requires nightly `#![feature(specialization)]`)

Resource arguments map to `impl P<hex><'bound, Error = Self::Error> + 'bound`; borrowed resources add `impl DerefMut<Target = ...> + 'bound`; nullable resources wrap in `Option<...>`.

### pit-c-generic / pit-go-generic / pit-haxe-generic / pit-swift-generic / pit-ts-generic

These are thin backwards-compatibility re-exports of `pit-lang-generic`. New code should depend on `pit-lang-generic` directly.

### pit-to-capnp

Stub crate. Defines:
- `trait Capnp` — one method `capnp(&self, &mut Formatter) -> fmt::Result`
- `struct ViaCapnp<'a>` — `Display` wrapper that calls `Capnp::capnp`

No actual PIT-to-Cap'n-Proto conversion logic is implemented yet.

### pit-wit-bridge

Stub crate. Defines:
- `trait ToWIT` — one method `to_wit(&self, &mut Formatter) -> fmt::Result`
- `struct ViaWIT<'a>` — `Display` wrapper that calls `ToWIT::to_wit`

No actual PIT-to-WIT conversion logic is implemented yet.

## Common PIT interface files (`pit/common/`)

Example interface definitions bundled with the repo:

| File | Methods |
|---|---|
| `buffer.pit` | `read8(I32) -> (I32)`, `write8(I32, I32) -> ()`, `size() -> (I32)` — 32-bit addressed byte buffer |
| `buffer64.pit` | Same but with `I64` addresses |
| `reader.pit` | `read(I32) -> (buffer)`, `read64(I64) -> (buffer64)` — returns buffer resources by ID |
| `writer.pit` | `write(buffer) -> (I32)`, `write64(buffer64) -> (I64)` |

Resource cross-references in `reader.pit` and `writer.pit` use the raw 32-byte hex IDs of `buffer.pit` and `buffer64.pit`.

## Usage

```rust
use pit_lang_generic::{Opts, Go, TypeScript, TypeScriptAsync, Swift, C};
use pit_core::Interface;

let iface: Interface = /* parse from .pit source */;

// Go
let go_code = Opts::<Go>::default().interface(&iface).to_string();

// TypeScript (async)
let ts_code = Opts::<TypeScriptAsync>::default().interface(&iface).to_string();

// C with a custom prefix
let mut opts = Opts::<C>::default();
opts.syntax.prefix = "my_".into();
let c_code = opts.interface(&iface).to_string();
```

```rust
use pit_rust_generic::{Params, FeatureFlags, interface};
use pit_core::Interface;

let iface: Interface = /* … */;
let params = Params {
    core: syn::parse_quote!(::core),
    flags: FeatureFlags::default(),
    asyncness: None,
};
let tokens: proc_macro2::TokenStream = interface(&params, &iface);
```

## Feature flags

All crates share the same optional feature set:

| Flag | Effect |
|---|---|
| `unstable-sdk` | Enable `portal-solutions-sdk` integration |
| `unstable-pcode` | Enable pcode expression support in `pit-core` |
| `unstable-sdkcode` | Combined SDK + pcode |
| `unstable-generics` | Generic parameter support in `pit-core` |

## Building

```bash
cargo build
cargo doc --open
```

There are no tests (`cargo test` will run zero test cases).

## License

MPL-2.0

## Related

- [pit-core](https://github.com/portal-co/pit-core) — PIT parser and data structures (`Interface`, `Sig`, `Arg`, `ResTy`)
- [portal-solutions-sdk](https://github.com/portal-co/portal-solutions-sdk) — optional SDK integration behind feature flags
