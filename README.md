# more-pit

Code generation libraries for Portal Interface Types (PIT) across multiple programming languages.

This repository provides code generators that take PIT interface definitions and produce idiomatic bindings for various target languages. It extends the core PIT functionality defined in [pit-core](https://github.com/portal-co/pit-core).

## Overview

PIT (Portal Interface Types) is an Interface Definition Language (IDL) for defining cross-language interfaces. This repository contains language-specific code generators that convert PIT interfaces into native code for:

- **Rust** (`pit-rust-generic`) - Generates Rust traits with proc-macro2/quote
- **Go** (`pit-go-generic`) - Generates Go interfaces
- **TypeScript** (`pit-ts-generic`) - Generates TypeScript type definitions
- **Swift** (`pit-swift-generic`) - Generates Swift protocols
- **C** (`pit-c-generic`) - Generates C header macros
- **Haxe** (`pit-haxe-generic`) - Generates Haxe interfaces
- **Cap'n Proto** (`pit-to-capnp`) - Converts PIT to Cap'n Proto format
- **WIT** (`pit-wit-bridge`) - Bridges PIT to WebAssembly Interface Types (WIT)

## Quick Start

Add the desired crate(s) to your `Cargo.toml`:

```toml
[dependencies]
pit-rust-generic = { git = "https://github.com/portal-co/more-pit.git" }
pit-go-generic = { git = "https://github.com/portal-co/more-pit.git" }
# ... or any other crate
```

## PIT Interface Format

PIT interfaces follow this structure (see [pit-core SPEC.md](https://github.com/portal-co/pit-core/blob/main/SPEC.md) for details):

```
[attribute=value]{
    method_name(ParamType1, ParamType2) -> (ReturnType1, ReturnType2);
}
```

### Primitive Types
- `I32`, `I64` - 32/64-bit integers
- `F32`, `F64` - 32/64-bit floating point numbers

### Resource Types
- `R<hex_id>` - Reference to a resource by its 32-byte hex ID
- `this` - Self-reference to the current interface
- Resource modifiers: `n` (nullable), `&` (reference/borrowed)

## Common Interfaces

The `pit/common/` directory contains reusable PIT interface definitions:

- `buffer.pit` - 32-bit addressed byte buffer with read/write operations
- `buffer64.pit` - 64-bit addressed byte buffer
- `reader.pit` - Buffer reader abstraction
- `writer.pit` - Buffer writer abstraction

## Crate Documentation

Each crate in `crates/` provides language-specific code generation:

### pit-rust-generic
Generates Rust trait definitions using `proc-macro2` and `quote`. Supports async traits and specialization via feature flags.

### pit-go-generic
Generates Go interface definitions. Supports package rewrites for cross-package references.

### pit-ts-generic
Generates TypeScript type definitions. Supports async/Promise return types.

### pit-swift-generic
Generates Swift protocol definitions with existential types.

### pit-c-generic
Generates C header macros using the `vfunc` macro pattern for virtual function tables.

### pit-haxe-generic
Generates Haxe interface definitions. Supports package rewrites.

### pit-to-capnp
Converts PIT interfaces to Cap'n Proto schema format.

### pit-wit-bridge
Bridges PIT to WebAssembly Interface Types (WIT) format.

## Feature Flags

Most crates support these feature flags:
- `unstable-sdk` - Enable portal-solutions-sdk integration
- `unstable-pcode` - Enable pcode expression support
- `unstable-sdkcode` - Combined SDK and pcode support
- `unstable-generics` - Enable generic parameter support

## Building

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Generate documentation
cargo doc --open
```

## Requirements

- Rust nightly toolchain (see `rust-toolchain.toml`)

## License

MPL-2.0 (see individual crate Cargo.toml files)

## Related Projects

- [pit-core](https://github.com/portal-co/pit-core) - Core PIT parsing and data structures
- [portal-solutions-sdk](https://github.com/portal-co/portal-solutions-sdk) - Portal Solutions SDK
