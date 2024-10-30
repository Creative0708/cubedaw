use anyhow::Context;
use cubedaw_lib::{Buffer, Id, NodeEntry, Patch};

use crate::WorkerOptions;

use super::{NodeGraphEntry, PreparedNodeGraph, WorkerState};

#[derive(Debug, Clone)]
pub struct SynthNoteNodeGraph(PreparedNodeGraph);

impl SynthNoteNodeGraph {
    pub fn empty() -> Self {
        // TODO: set this to a basic node graph so input_node() and friends can't panic
        Self(PreparedNodeGraph::empty(None, Id::invalid()))
    }
    pub fn sync_with(&mut self, patch: &Patch, options: &WorkerOptions) -> anyhow::Result<()> {
        let mut note_output = None;

        for (id, node) in patch.nodes() {
            if node.tag() == cubedaw_lib::NodeTag::Special {
                let res = &node.data.key;
                if res == &resourcekey::literal!("builtin:note_output") {
                    assert!(
                        note_output.replace(id).is_none(),
                        "TODO handle multiple note outputs"
                    );
                }
            }
        }

        self.0.sync_with(
            patch,
            options,
            None,
            note_output.context("no note output exists")?,
        );

        Ok(())
    }

    pub fn process(&mut self, options: &WorkerOptions, state: &mut WorkerState) -> &mut Buffer {
        self.0.process(options, state);

        let output_node = self
            .0
            .get_node_mut(self.0.output_node())
            .expect("unreachable");
        &mut output_node.outputs[0]
    }
}
