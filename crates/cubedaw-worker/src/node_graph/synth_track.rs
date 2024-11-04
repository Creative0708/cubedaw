use anyhow::{Context, Result};
use cubedaw_lib::{Buffer, Id, Patch};

use crate::WorkerOptions;

use super::{PreparedNodeGraph, WorkerState};

#[derive(Debug, Clone)]
pub struct SynthTrackNodeGraph(PreparedNodeGraph);

impl SynthTrackNodeGraph {
    pub fn empty() -> Self {
        Self(PreparedNodeGraph::empty(None, Id::invalid()))
    }
    pub fn sync_with(&mut self, patch: &Patch, options: &WorkerOptions) -> anyhow::Result<()> {
        let mut track_output = patch
            .get_active_node(&resourcekey::literal!("builtin:track_output"))
            .context("no track output exists")?;
        let mut note_output = patch
            .get_active_node(&resourcekey::literal!("builtin:note_output"))
            .context("no note output exists")?;

        self.0
            .sync_with(patch, options, Some(note_output), track_output);

        self.0
            .get_node_mut(track_output)
            .expect("unreachable")
            .outputs = vec![Buffer::new_box_zeroed(options.buffer_size)];

        Ok(())
    }

    pub fn process(
        &mut self,
        options: &WorkerOptions,
        state: &mut WorkerState,
        input: &Buffer,
    ) -> Result<&mut Buffer> {
        let input_node = self
            .0
            .get_node_mut(self.0.input_node().expect("unreachable"))
            .expect("unreachable");
        input_node.outputs[0].copy_from(input);

        self.0.process(options, state);

        let output_node = self
            .0
            .get_node_mut(self.0.output_node())
            .expect("unreachable");
        Ok(&mut output_node.outputs[0])
    }
}
