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
        let mut track_output = None;
        let mut note_output = None;

        for (id, node) in patch.nodes() {
            if node.tag() == cubedaw_lib::NodeTag::Special {
                let res = &node.data.key;
                if res == &resourcekey::literal!("builtin:track_output") {
                    assert!(
                        track_output.replace(id).is_none(),
                        "TODO handle multiple track outputs"
                    );
                } else if res == &resourcekey::literal!("builtin:note_output") {
                    assert!(
                        note_output.replace(id).is_none(),
                        "TODO handle multiple note outputs"
                    );
                }
            }
        }

        let note_output = note_output.context("no note output exists")?;
        let track_output = track_output.context("no track output exists")?;

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
