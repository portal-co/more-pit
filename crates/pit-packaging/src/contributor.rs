use anyhow::Result;
use pit_core::Interface;

use crate::artifact::EmittedArtifact;
use crate::graph::PitGraph;

pub trait PackagingContributor<C: ?Sized> {
    fn id(&self) -> &'static str;
    fn emit(&self, iface: &Interface, graph: &PitGraph, cfg: &C) -> Result<Vec<EmittedArtifact>>;
}

pub struct ContributorRegistry<C: ?Sized> {
    contributors: Vec<Box<dyn PackagingContributor<C>>>,
}

impl<C: ?Sized> Default for ContributorRegistry<C> {
    fn default() -> Self {
        Self {
            contributors: Vec::new(),
        }
    }
}

impl<C: ?Sized> ContributorRegistry<C> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, contributor: impl PackagingContributor<C> + 'static) {
        self.contributors.push(Box::new(contributor));
    }

    pub fn contributors(&self) -> &[Box<dyn PackagingContributor<C>>] {
        &self.contributors
    }
}
