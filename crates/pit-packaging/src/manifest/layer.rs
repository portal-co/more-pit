use anyhow::Result;
use pit_core::Interface;

use crate::artifact::EmittedArtifact;
use crate::graph::PitGraph;
use crate::manifest::merge::ManifestFragment;

pub struct ManifestContext<'a, C: ?Sized> {
    pub iface: &'a Interface,
    pub graph: &'a PitGraph,
    pub artifacts: &'a [EmittedArtifact],
    pub cfg: &'a C,
    pub prior: &'a [ManifestFragment],
}

pub trait ManifestLayer<C: ?Sized> {
    fn id(&self) -> &'static str;
    fn emit(&self, ctx: &ManifestContext<'_, C>) -> Result<Vec<ManifestFragment>>;
}

pub trait NestableManifestLayer<C: ?Sized>: ManifestLayer<C> {
    fn children(&self) -> &[Box<dyn ManifestLayer<C>>];
}

pub struct ManifestPipeline<C: ?Sized> {
    layers: Vec<Box<dyn ManifestLayer<C>>>,
}

impl<C: ?Sized> Default for ManifestPipeline<C> {
    fn default() -> Self {
        Self { layers: Vec::new() }
    }
}

impl<C: ?Sized> ManifestPipeline<C> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nest(mut self, layer: impl ManifestLayer<C> + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    pub fn run(&self, ctx: ManifestContext<'_, C>) -> Result<Vec<ManifestFragment>> {
        let mut all = Vec::new();
        for layer in &self.layers {
            let child_ctx = ManifestContext {
                prior: &all,
                ..ctx
            };
            let mut emitted = layer.emit(&child_ctx)?;
            all.append(&mut emitted);
        }
        Ok(all)
    }
}
