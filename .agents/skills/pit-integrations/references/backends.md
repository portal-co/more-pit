# Current PIT backend reference

This reference summarizes checked-in mappings. Verify behavior in source before changing a backend. A PIT binding backend and a value-conversion shim are separate support claims.

## Shared `pit-lang-generic::Syntax` backends

| Target | Integer mapping | Resource/nullable behavior | Notes |
|---|---|---|---|
| C | Emitted through `interface99` macros | Macro-level; inspect `crates/pit-lang-generic/src/c.rs` | `C.prefix` customizes type names. File output includes `stdint.h`, `interface99.h`, and dependency headers. |
| Go | `I32 → uint32`, `I64 → uint64` | Resource refs map to `P<hex>`; Go nilability makes nullable flag implicit. Borrow/take ignored. | Files use `package pitbindings`; rewrites can qualify cross-package refs. |
| Haxe | `I32 → haxe.Int32`, `I64 → haxe.Int64` | Nullable resources use `Null<T>`; borrow/take ignored. | Files use `package pit`. |
| TypeScript | `I32 → number`, `I64 → bigint`, floats → `number` | Nullable resources use `T | undefined`; borrow/take ignored. | Multiple returns are tuples; sync interface names are `P<hex>`. |
| TypeScript async | Same as TypeScript | Resource names use `AP<hex>`. | Methods return `void | Promise<void>` or tuple/Promise unions. |
| Swift | `I32 → UInt32`, `I64 → UInt64`, floats → `Float`/`Double` | Nullable resources use optionals; borrow/take ignored; `This → Self`. | Files are compiled together. |
| Haskell | `I32 → Word32`, `I64 → Word64`, floats → `Float`/`Double` | Nullable resources use `Maybe`; borrow/take ignored. | Monad-generic classes, associated resource type `Self m`; dependencies are qualified imports. |
| Java | `I32 → int`, `I64 → long`, floats → `float`/`double` | Nullable refs use `java.util.Optional<T>`; borrow/take ignored. | Current emitter maps multiple returns to `Object`; this is a representational limitation. |
| Scala | `I32 → Int`, `I64 → Long`, floats → `Float`/`Double` | Nullable resources use `Option[T]`; borrow/take ignored. | Emits traits in the configured package. |

The C, Go, Haxe, TypeScript, Swift, Haskell, Java, and Scala types are implemented in `crates/pit-lang-generic/src/lib.rs`; C's formatting is in `src/c.rs`.

## Separate and specialized generators

- **Rust:** `pit-rust-generic` emits traits via `proc_macro2`/`quote`. It maps `I32`/`I64` to `u32`/`u64`; resource types become trait bounds. `ResTy::None` and unsupported types have special/error mappings; inspect `crates/pit-rust-generic/src/lib.rs` before assuming exhaustive support.
- **Scala-C:** `pit-scala-c-bridge` handles Scala native/C interoperability and is not the ordinary Scala `Syntax` backend.
- **JS TeaVM:** `pit-js-teavm` emits TeaVM-specific glue. It is not a generic WebAssembly backend, and TeaVM glue remains outside `pit-lang-generic::wire`.

## SDK↔PIT value-conversion shims

| Shim target | Generator | Host responsibility | Validation |
|---|---|---|---|
| Rust | `pit-sdk-gen --backend rust` / `emit_rust_shim` | Implement `ResourceResolver<T>` for opaque aggregates and interface objects | Generated Rust compile test plus runtime converter tests |
| TypeScript | `pit-sdk-gen --backend ts` / `emit_typescript_shim` | Implement `ResourceResolver<T>` for opaque aggregates and interface objects | Strict `tsc` compile and Node round-trip/range test |

These are paired SDK↔PIT value converters, not PIT binding backends. Both consume `Info` lowering markers and reject unknown versions; only Rust and TypeScript currently claim this shim surface. PIT-only generation remains dispatched by `pit-gen`.

`pit-gen` is the source of truth for selectable PIT binding names and dispatch. Compile-test coverage is governed by `docs/compile-tests.md`; adding a backend or shim requires a hard-fail compile test for the claimed output.
