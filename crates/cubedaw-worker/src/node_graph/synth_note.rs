use anyhow::{Context, Result};
use cubedaw_lib::{Buffer, Id, InternalBufferType, Note, Patch};

use crate::{
    WorkerOptions,
    plugin::{Attribute, AttributeMap},
};

use super::{PreparedNodeGraph, WorkerState};

#[derive(Debug, Clone)]
pub struct NoteNodeGraph(PreparedNodeGraph);

impl NoteNodeGraph {
    pub fn empty() -> Self {
        // TODO: set this to a basic node graph so input_node() and friends can't panic
        Self(PreparedNodeGraph::empty(None, Id::invalid()))
    }
    pub fn sync_with(&mut self, patch: &Patch, options: &WorkerOptions) -> anyhow::Result<()> {
        let note_output = patch
            .get_active_node(&resourcekey::literal!("builtin:output"))
            .context("no note output exists")?;

        self.0.sync_with(patch, options, None, note_output);

        Ok(())
    }

    pub fn process(
        &mut self,
        options: &WorkerOptions,
        state: &mut WorkerState,
        note: &Note,
    ) -> Result<&Buffer> {
        struct NoteAttributeMap<'a> {
            note: &'a Note,
        }
        impl<'a> AttributeMap for NoteAttributeMap<'a> {
            fn attribute(&self, attr: Attribute) -> InternalBufferType {
                match attr {
                    Attribute::Pitch => InternalBufferType::splat(self.note.pitch as f32 / 12.0),
                }
            }
        }

        self.0
            .process(options, state, &mut NoteAttributeMap { note })?;

        let output_node = self
            .0
            .get_node_mut(self.0.output_node())
            .expect("unreachable");
        Ok(&output_node.outputs[0].buffer)
    }
}
