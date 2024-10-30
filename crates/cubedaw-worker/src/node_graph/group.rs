use anyhow::Context;
use cubedaw_lib::{Buffer, Id, NodeEntry, Patch};

use crate::{WorkerOptions, WorkerState};

use super::{NodeGraphEntry, PreparedNodeGraph};

#[derive(Debug, Clone)]
pub struct GroupNodeGraph(PreparedNodeGraph);

impl GroupNodeGraph {
    pub fn empty() -> Self {
        Self(PreparedNodeGraph::empty(None, Id::invalid()))
    }
    pub fn sync_with(&mut self, patch: &Patch, options: &WorkerOptions) -> anyhow::Result<()> {
        let mut track_input = None;
        let mut track_output = None;

        for (id, node) in patch.nodes() {
            if node.tag() == cubedaw_lib::NodeTag::Special {
                let res = &node.data.key;
                if res == &resourcekey::literal!("builtin:track_input") {
                    assert!(
                        track_input.replace(id).is_none(),
                        "TODO handle multiple track outputs"
                    );
                } else if res == &resourcekey::literal!("builtin:track_output") {
                    assert!(
                        track_output.replace(id).is_none(),
                        "TODO handle multiple note outputs"
                    );
                }
            }
        }

        self.0.sync_with(
            patch,
            options,
            Some(track_output.context("no note output exists")?),
            track_input.context("no track output exists")?,
        );

        Ok(())
    }

    pub fn process(
        &mut self,
        options: &WorkerOptions,
        state: &mut WorkerState,
        input: &Buffer,
    ) -> &mut Buffer {
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
        &mut output_node.outputs[0]
    }
}
