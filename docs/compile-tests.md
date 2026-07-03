# Compile test policy

This document defines how compile tests behave in the **more-pit** repository. It is the single source of truth for compile-test requirements — see [`AGENTS.md`](../AGENTS.md) for agent-specific instructions.

## Policy

Compile tests invoke real language toolchains against generated output. They **must not** skip when a toolchain is missing.

| Outcome | Meaning |
|---------|---------|
| Toolchain missing | Test **panics** with an install hint — same severity as a compile failure |
| Toolchain present, codegen broken | Test **fails** with compiler diagnostics |
| Toolchain present, codegen correct | Test **passes** |

There is no `SKIP`, early return, or `#[ignore]` escape hatch for compile tests. A green `cargo test -p pit-gen` means every compile test had its toolchain available and the generated code type-checked.

Structural and unit tests (Rust-only, no external compiler) may run without the toolchains below. Compile tests may not.

## `require_toolchain` contract

Compile tests use a helper that panics when a program is not on `PATH`:

```rust
fn require_toolchain(program: &str, install_hint: &str) -> PathBuf {
    which(program).unwrap_or_else(|| {
        panic!(
            "compile test requires `{program}` on PATH — {install_hint}\n\
             See docs/compile-tests.md"
        );
    })
}
```

Variants (e.g. `require_tsc()`) follow the same rule: **panic, never skip**.

Agents and contributors must not weaken this pattern to restore silent skips.

## Toolchains (more-pit)

| Tool | Used by | Typical install |
|------|---------|-----------------|
| `javac` | `test_java_type_checks` | JDK (`brew install openjdk`, `apt install openjdk-17-jdk`, …) |
| `scalac` | `test_scala_type_checks` | Scala (`brew install scala`, SDKMAN, …) |
| `cc` or `clang` | `test_c_type_checks` | Xcode CLI tools, `build-essential`, … |
| `go` | `test_go_type_checks` | [go.dev](https://go.dev/dl/) or `brew install go` |
| `haxe` | `test_haxe_type_checks` | [haxe.org](https://haxe.org/download/) or `brew install haxe` |
| `tsc` | TypeScript, ts-async, and js-teavm adapter compile tests | `npm ci` at repo root (local `node_modules/.bin/tsc`) or global TypeScript |
| `javac` | `pit-js-teavm` Java glue compile test (`pit-js-teavm/tests/compile_tests.rs`) | JDK (`brew install openjdk`, …) |
| `scalac` | `pit-js-teavm` Scala glue compile test (`pit-js-teavm/tests/compile_tests.rs`) | Scala (`brew install scala`, …) |
| `swiftc` | `test_swift_type_checks` | Xcode / Swift toolchain |
| `ghc` | `test_haskell_type_checks` | GHCup or `brew install ghc` |

Prefer `cc` over `clang` when both are present (matches existing test logic).

## Running locally

```bash
cargo test --workspace
cargo test -p pit-gen -- --nocapture
```

Install the full toolchain set above before expecting a fully green run.

## CI

CI workflows must install every toolchain listed in this document. Compile steps are not optional matrix cells — missing tools are build failures, not skipped jobs.
