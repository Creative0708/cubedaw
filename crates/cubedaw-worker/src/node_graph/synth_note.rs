use anyhow::{Context, Result};
use cubedaw_lib::{Buffer, Id, Patch};

use crate::WorkerOptions;

use super::{PreparedNodeGraph, WorkerState};

#[derive(Debug, Clone)]
pub struct SynthNoteNodeGraph(PreparedNodeGraph);

impl SynthNoteNodeGraph {
    pub fn empty() -> Self {
        // TODO: set this to a basic node graph so input_node() and friends can't panic
        Self(PreparedNodeGraph::empty(None, Id::invalid()))
    }
    pub fn sync_with(&mut self, patch: &Patch, options: &WorkerOptions) -> anyhow::Result<()> {
        let mut note_output = patch
            .get_active_node(&resourcekey::literal!("builtin:note_output"))
            .context("no note output exists")?;

        self.0.sync_with(patch, options, None, note_output);

        Ok(())
    }

    pub fn process(&mut self, options: &WorkerOptions, state: &mut WorkerState) -> Result<&Buffer> {
        self.0.process(options, state)?;

        let output_node = self
            .0
            .get_node_mut(self.0.output_node())
            .expect("unreachable");
        Ok(&output_node.outputs[0].buffer)
    }
}
