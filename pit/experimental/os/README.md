# OS Environment PIT Interfaces (Experimental)

Capability-based PIT interface definitions covering the five OS environment
capability groups from `os-env-traits`: `FileEnv`, `GitEnv`, `GitHubEnv`,
`NetworkEnv`, and `AiEnv`.

> **Experimental status:** these interfaces have not been used in production.
> Their definitions — and therefore their RIDs — may still change before first
> use. Once any interface is referenced by production code, it is permanently
> frozen. See "RID Permanence" below.

## Type-System Conventions

PIT has no string, bool, or struct types. All OS-level data is represented
through resource interfaces:

| Concept | PIT encoding |
|---------|-------------|
| String or bytes | `R<buffer-rid>` owned (output) / `R<buffer-rid>&` borrowed (input) |
| `Option<String>` | `R<buffer-rid>n` (nullable owned buffer) |
| `bool` | `I32` — 0 = false, nonzero = true |
| `Vec<String>` | `R<string-list-rid>` — list interface with `len()` + `get(I32)` |
| `Vec<T>` | dedicated list interface with `len()` + `get(I32)` |
| stateful iterator | walker interface with `next() -> (R<item>n)` — null signals end |
| struct with fields | dedicated interface with one getter per field |

Borrowed (`&`) resources are used for read-only inputs. Owned resources are
used for outputs that the caller takes responsibility for.

## Files and RIDs

### Data-type interfaces

These model structured data shapes. They have no side effects on their own —
they're purely descriptors of what a capability interface returns or accepts.

| File | RID | Description |
|------|-----|-------------|
| `string-list.pit` | `ccd39e387d198ec0c150c4597f3af76b3c6f78bb0b8bfbd89971434e57ab851c` | Random-access list of string buffers (`len` + `get`) |
| `dir-entry.pit` | `14bee9f61128f9cfda108998b4b993c9173d0ad38245a8e0a38fdf46b5527344` | Single directory entry: `path` (buffer) + `is_dir` (I32) |
| `dir-walker.pit` | `698e0c782abc6e6b3da1148b608d3223f38df5b833b953a7db31651ddcb8bfb2` | Iterator over `dir-entry` resources; `next()` returns nullable entry |
| `github-file.pit` | `0e2aa622c1826242bfecea7d85c439bba2f48845c79cf8a92133da4d275e2626` | GitHub Contents API file entry: `name`, `path`, `kind`, optional `download_url` |
| `github-file-list.pit` | `6280a44102953a99d250e16b703579709323d50bd2b50a204d0b5a63e60141a1` | Random-access list of `github-file` resources |

### Capability interfaces

Each corresponds to one capability group in `os-env-traits`. A component that
needs file access accepts `R<file-env-rid>` as a parameter rather than having
that access ambient — this is the capability-based pattern.

| File | RID | Corresponds to |
|------|-----|----------------|
| `file-env.pit` | `c3f60478ed87d7d522c8e711836689c874df0a254a9763b3542de7259784d1be` | `FileEnv` — read, write, exists, walk, env_var |
| `git-env.pit` | `5064fc3dfd61c1de246e9f2f29bfefe8c700d5c15a25c430671448edd46ef5e8` | `GitEnv` — repo_root, rev_parse, show_file, changed_files, etc. |
| `network-env.pit` | `5ecf45a1cace9552cb5794185068976d95252e02f1f2f22633626f973768632c` | `NetworkEnv` — get, post_json |
| `github-env.pit` | `9cf620742b025f89429f437163db44cd1c22add25fbc0c7d966c177401f820b4` | `GitHubEnv` — current_owner, list_repos, list_contents, download_file |
| `ai-env.pit` | `7a2ffa63ae1d6a74378940fbb3da332cc75e50c7fab9586675c3f752a8ffaceb` | `AiEnv` — scan(path, content) → (likely_ai: I32, confidence: F64) |

## Computing and Verifying RIDs

Use the `pit-rid` CLI in this workspace:

```sh
# Single interface — bare hex output
cargo run -p pit-rid -- pit/experimental/os/file-env.pit

# All at once — md5sum-style output
cargo run -p pit-rid -- pit/experimental/os/*.pit
```

Cross-references between interfaces embed the 64-char hex RID of the
dependency directly in the file. See `dir-walker.pit` (references `dir-entry`)
and `file-env.pit` (references `buffer` and `dir-walker`) for examples.

If you edit an experimental interface and need to update dependent files:
1. Edit the interface file
2. Run `pit-rid` on it to get the new RID
3. Find all files that embed the old RID and substitute the new one
4. Re-run `pit-rid` on dependents to confirm their RIDs have also changed

## RID Permanence — No Versioning System

**There is no versioning system in PIT. This is a deliberate design choice.**

A RID is derived from the interface's content. It is the interface's permanent
identity. The implications:

- **Once used in production, an interface is frozen.** It must be supported
  indefinitely, even after deprecation.
- **To evolve, define a new interface.** It will have a new RID. Old consumers
  continue to work; new consumers use the new RID.
- **Experimental interfaces are still free to change** — these files have not
  been used in production yet. The `[experimental=true]` attribute signals
  this and is part of the content-addressed identity.
- **Promotion to stable = new file, new RID.** Copy the interface to
  `pit/common/` or `pit/stable/`, remove the `[experimental=true]` attribute.
  The result is a different RID — a deliberate commitment to that exact form.

The absence of versioning eliminates "v2 churn": there is no pressure to
number versions or maintain a version history. Interfaces that are good enough
to use are good enough to keep forever.
