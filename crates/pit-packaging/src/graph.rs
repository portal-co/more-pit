use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pit_core::Interface;
use walkdir::WalkDir;

/// One interface discovered under a `pit/` tree.
#[derive(Clone, Debug)]
pub struct PitGraphEntry {
    pub iface: Interface,
    /// Path relative to `pit/` root (for mirroring rust output layout).
    pub rel_path: PathBuf,
}

/// All interfaces indexed by RID.
#[derive(Clone, Debug, Default)]
pub struct PitGraph {
    pub entries: BTreeMap<[u8; 32], PitGraphEntry>,
}

impl PitGraph {
    pub fn interfaces(&self) -> BTreeMap<[u8; 32], Interface> {
        self.entries
            .iter()
            .map(|(k, e)| (*k, e.iface.clone()))
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&[u8; 32], &PitGraphEntry)> {
        self.entries.iter()
    }
}

pub fn find_pit_root(input: &Path) -> PathBuf {
    let mut dir = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        if dir.file_name().is_some_and(|name| name == "pit") {
            return dir;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
    input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn collect_pit_graph(pit_dir: &Path) -> PitGraph {
    let mut graph = PitGraph::default();
    if !pit_dir.is_dir() {
        return graph;
    }
    for entry in WalkDir::new(pit_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pit") {
            continue;
        }
        let Ok(a) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok((_, iface)) = pit_core::parse_interface(&a) else {
            continue;
        };
        let rel = path.strip_prefix(pit_dir).unwrap_or(path).to_path_buf();
        graph.entries.insert(
            iface.rid(),
            PitGraphEntry {
                iface,
                rel_path: rel,
            },
        );
    }
    graph
}

/// Build a graph from a single interface (package mode).
pub fn single_iface_graph(iface: Interface, rel_path: PathBuf) -> PitGraph {
    let mut graph = PitGraph::default();
    graph.entries.insert(
        iface.rid(),
        PitGraphEntry {
            iface,
            rel_path,
        },
    );
    graph
}
