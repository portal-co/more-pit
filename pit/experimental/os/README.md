# OS Environment PIT Interfaces (Experimental)

Capability-based PIT interface definitions covering the five OS environment
capability groups from `os-env-traits`: `FileEnv`, `GitEnv`, `GitHubEnv`,
`NetworkEnv`, and `AiEnv`.

> **Experimental status:** these interfaces have not been used in production.
> Their definitions — and therefore their RIDs — may still change before first
> use. Once any interface is referenced by production code, it is permanently
> frozen. See "RID Permanence" below.

## Capability Design

The filesystem interfaces are designed without path strings. Instead of passing
`"/some/path/to/file"` to a capability, callers navigate the directory tree by
object:

1. Obtain a root `dir-entry` from `file-env.root()`
2. Call `dir-entry.get(name)` to get a child by name (returns nullable `dir-entry`)
3. Call `dir-entry.child_at(i)` / `dir-entry.child_count()` to iterate children
4. Call `dir-entry.navigate(path_segments)` for multi-level navigation via a
   `string-list` of name segments
5. Call `dir-entry.read()` / `dir-entry.write(...)` on a file node

A `dir-entry` is a unified filesystem node — it represents both files and
directories. Check `is_dir()` before calling content or navigation methods.
Methods that don't apply (e.g. `read()` on a directory) return null or are no-ops.

This design prevents capability amplification: a component holding a `dir-entry`
for one subtree cannot escape to an arbitrary path.

## Type-System Conventions

PIT has no string, bool, or struct types. All OS-level data is represented
through resource interfaces:

| Concept | PIT encoding |
|---------|-------------|
| String or bytes | `R<buffer-rid>` owned (output) / `R<buffer-rid>&` borrowed (input) |
| `Option<String>` | `R<buffer-rid>n` (nullable owned buffer) |
| `bool` | `I32` — 0 = false, nonzero = true |
| `Vec<String>` / path segments | `R<string-list-rid>` — list with `len()` + `get(I32)` |
| `Vec<T>` | dedicated list interface with `len()` + `get(I32)` |
| struct with fields | dedicated interface with one getter per field |
| self-referential type | `Rthis` — the `this` keyword references the current interface |

Borrowed (`&`) resources are used for read-only inputs. Owned resources are
used for outputs that the caller takes responsibility for.

## Files and RIDs

### Data-type interfaces

| File | RID | Description |
|------|-----|-------------|
| `string-list.pit` | `ccd39e387d198ec0c150c4597f3af76b3c6f78bb0b8bfbd89971434e57ab851c` | Random-access list of string buffers — also used as path segment lists |
| `dir-entry.pit` | `3a2f017954c7ee45c9749cddde6892bbce0fd36680249371f3e68692676ace4a` | Unified filesystem node (file or dir); uses `Rthis` for all sub-entry references |
| `github-file.pit` | `0e2aa622c1826242bfecea7d85c439bba2f48845c79cf8a92133da4d275e2626` | GitHub Contents API file entry: `name`, `path`, `kind`, optional `download_url` |
| `github-file-list.pit` | `6280a44102953a99d250e16b703579709323d50bd2b50a204d0b5a63e60141a1` | Random-access list of `github-file` resources |

### Capability interfaces

| File | RID | Corresponds to |
|------|-----|----------------|
| `file-env.pit` | `4a4dd21c695a9abfe570537bc304a76edea8935bbf08732ef07d6de8dd443b0f` | `FileEnv` — `root()` returns the root `dir-entry`; `env_var()` for process env |
| `git-env.pit` | `5064fc3dfd61c1de246e9f2f29bfefe8c700d5c15a25c430671448edd46ef5e8` | `GitEnv` — repo_root, rev_parse, show_file, changed_files, etc. |
| `network-env.pit` | `5ecf45a1cace9552cb5794185068976d95252e02f1f2f22633626f973768632c` | `NetworkEnv` — get, post_json |
| `github-env.pit` | `9cf620742b025f89429f437163db44cd1c22add25fbc0c7d966c177401f820b4` | `GitHubEnv` — current_owner, list_repos, list_contents, download_file |
| `ai-env.pit` | `7a2ffa63ae1d6a74378940fbb3da332cc75e50c7fab9586675c3f752a8ffaceb` | `AiEnv` — scan(path, content) → (likely_ai: I32, confidence: F64) |

Note: `git-env`, `network-env`, `github-env`, and `ai-env` still accept string
buffers for URL and path arguments because they operate outside the local
filesystem capability boundary (remote systems, process environment).

## `dir-entry` Navigation Patterns

### Get a file by name
```
// root is a dir-entry resource
let file = root.get("config.toml")   // -> dir-entry (nullable)
let contents = file.read()           // -> buffer (nullable if dir)
```

### Iterate children
```
let n = root.child_count()
for i in 0..n:
    let entry = root.child_at(i)
    let name = entry.name()
    let is_dir = entry.is_dir()
```

### Multi-level navigation via path segment list
```
// navigate(["src", "main", "lib.rs"]) descends dir by dir, returns final entry
let entry = root.navigate(segments)  // -> dir-entry (nullable if any segment missing)
```

### Create and write a file
```
let file = dir.create_file("output.txt")   // -> dir-entry
file.write(contents_buffer)
```

## Computing and Verifying RIDs

```sh
# Single interface
cargo run -p pit-rid -- pit/experimental/os/dir-entry.pit

# All at once
cargo run -p pit-rid -- pit/experimental/os/*.pit
```

If you edit an experimental interface:
1. Run `pit-rid` on it to get the new RID
2. Update any files that embed the old RID
3. Re-run `pit-rid` on dependents to confirm their RIDs also changed

## RID Permanence — No Versioning System

**There is no versioning system in PIT. This is a deliberate design choice.**

- Once used in production, an interface is frozen and must be supported indefinitely.
- Evolution means a new file with a new RID; old consumers keep working.
- Experimental interfaces (here) are free to change — they haven't been used in production.
- Promotion to stable = new file in `pit/common/` without `[experimental=true]`, producing a new RID — a deliberate commitment.

The absence of versioning eliminates churn: an interface good enough to deploy is good enough to keep forever.
