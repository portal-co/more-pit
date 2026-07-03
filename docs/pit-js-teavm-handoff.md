# pit-js-teavm handoff (future)

Design notes for a future **`pit-js-teavm`** crate in more-pit. This document is standalone — not linked from `AGENTS.md`.

## Purpose

Provide **object-to-object** bridging between JavaScript/TypeScript host objects and guest `P{rid}` implementations, without going through a WASM wire layer.

## Scope

| In scope | Out of scope |
|----------|--------------|
| JS/TS object adapters for `P{rid}` method calls | WASM module export/import symbols |
| Reuse of `pit-lang-generic` guest `Syntax::TypeScript` types where applicable | `WireLayout` export/import emission |
| `pit-gen` backend sketch (`--backend js-teavm` or similar) | `tpit/` module paths or TeaVM Structure classes |
| Hard-fail compile tests (`tsc`) once implemented | Porting TeaVM Java Structure model to JS |

## Relationship to existing crates

- **`pit-teavm`** (pit repo): JVM bytecode → WASM via TeaVM; uses `@Import`/`@Export` and handle tables. Duplication with `pit-js-teavm` is acceptable.
- **`pit-lang-generic::TypeScript` / `TypeScriptAsync`**: canonical sync/async TS interface types (`P{rid}`, `AP{rid}`).
- **`pit-ts-host-core`** (pit repo): WASM host adapters — different axis (host calls into guest wasm), not object-to-object JS guest glue.

## Target architecture (sketch)

```
PIT interface
     │ pit-lang-generic Syntax::TypeScript
     ▼
P{rid} type (canonical)
     │ pit-js-teavm adapter layer
     ▼
JS object implementing P{rid} methods (plain functions / class)
     ↔ host JS object (no WASM intermediate)
```

## Non-goals

- No WASM wire layer, `WireLayout` symbol names, or `tpit/` imports.
- No requirement to share file format with `pit-teavm` or `pit-scala-c-bridge`.
- No `@Import`/`@Export` or TeaVM runtime dependencies.

## Compile-test expectations (when implemented)

- `tsc --noEmit` on generated adapter + fixture impl (same hard-fail policy as [`compile-tests.md`](compile-tests.md)).
- Optional Node execution test for a reference impl (e.g. buffer slice) — separate from compile policy.

## Open questions

- Package layout (`@portal/pit-js-teavm` vs generated per-interface files).
- Async vs sync: mirror `TypeScriptAsync` (`AP{rid}`) or JS-only Promise wrappers.
- Whether to share test fixtures with `impl/ts/` canonical implementations.
