# Agent instructions (more-pit)

Instructions for coding agents working in this repository.

## Compile tests

**Read and follow [`docs/compile-tests.md`](docs/compile-tests.md).** That document is the compile-test policy. Do not restate or weaken it here.

When compile tests fail because a toolchain is missing:

1. **Detect** which tool is absent (`javac`, `scalac`, `cc`, `go`, `haxe`, `tsc`, `swiftc`, `ghc`, …).
2. **Flag** clearly in your response which tools are missing.
3. **Attempt install** where reasonable (e.g. `brew install openjdk`, `brew install go`, `npm ci` at repo root for `tsc`).
4. **Never revert** hard-fail policy — no `SKIP`, early return, `#[ignore]`, or optional compile tests.
5. **Never stub or delete** compile tests to avoid installing toolchains.

If tests fail due to **broken codegen**, fix the emitter — not the policy.

## Architecture guardrails

| Concern | Location | Rule |
|---------|----------|------|
| Wire layout (abstract) | `pit-lang-generic::wire` | PIT → field layout only. **No TeaVM, WASM, or @Import/@Export.** |
| Canonical `P{rid}` interfaces | `pit-gen` + `pit-lang-generic` | Pure interfaces via `Syntax::*` backends. |
| Scala ↔ C bridge | `pit-scala-c-bridge` (future) | Implements `FfiBackend` for Scala native FFI — C structs, Scala `@extern`. **Not** TeaVM, **not** WASM. |
| TeaVM WASM glue | **pit** `pit-teavm` | Stays in the pit repo; more-pit does not emit WASM glue. |

**Do not** put TeaVM types or annotations in `pit-lang-generic::wire`. **Do not** move WASM TeaVM emission into more-pit.

When adding a new language backend, add a matching hard-fail compile test and extend [`docs/compile-tests.md`](docs/compile-tests.md).
