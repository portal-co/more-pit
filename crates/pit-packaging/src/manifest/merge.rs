use anyhow::Result;
use std::collections::BTreeMap;

/// Strategy for combining a new fragment with any existing content at `path`.
pub trait ManifestMerge: Send + Sync {
    fn id(&self) -> &'static str;
    fn apply(&self, path: &str, incoming: &str, existing: Option<&str>) -> Result<String>;
}

/// One emitted manifest contribution.
pub struct ManifestFragment {
    pub path: String,
    pub content: String,
    pub merge: Box<dyn ManifestMerge>,
}

/// Fold fragments in order, dispatching `merge.apply` per path.
pub fn commit_manifests(fragments: &[ManifestFragment]) -> Result<BTreeMap<String, String>> {
    let mut store: BTreeMap<String, String> = BTreeMap::new();
    for fragment in fragments {
        let existing = store.get(&fragment.path).map(|s| s.as_str());
        let merged = fragment
            .merge
            .apply(&fragment.path, &fragment.content, existing)?;
        store.insert(fragment.path.clone(), merged);
    }
    Ok(store)
}

/// Write committed manifest files to `out`.
pub fn write_manifests(out: &std::path::Path, manifests: &BTreeMap<String, String>) -> Result<()> {
    for (path, content) in manifests {
        let full = out.join(path);
        if let Some(parent) = full.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(full, content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Replace;

    impl ManifestMerge for Replace {
        fn id(&self) -> &'static str {
            "replace"
        }
        fn apply(&self, _path: &str, incoming: &str, _existing: Option<&str>) -> Result<String> {
            Ok(incoming.to_string())
        }
    }

    struct Append;

    impl ManifestMerge for Append {
        fn id(&self) -> &'static str {
            "append"
        }
        fn apply(&self, _path: &str, incoming: &str, existing: Option<&str>) -> Result<String> {
            Ok(match existing {
                Some(e) => format!("{e}\n{incoming}"),
                None => incoming.to_string(),
            })
        }
    }

    #[test]
    fn commit_applies_merge_in_order() {
        let fragments = vec![
            ManifestFragment {
                path: "a.txt".into(),
                content: "hello".into(),
                merge: Box::new(Replace),
            },
            ManifestFragment {
                path: "a.txt".into(),
                content: "world".into(),
                merge: Box::new(Append),
            },
        ];
        let out = commit_manifests(&fragments).unwrap();
        assert_eq!(out.get("a.txt").map(String::as_str), Some("hello\nworld"));
    }
}
