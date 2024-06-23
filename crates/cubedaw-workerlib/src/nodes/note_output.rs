use cubedaw_lib::{NodeState, NodeUiContext};
use cubedaw_node::{Node, NodeContext};

use crate::Buffer;

#[derive(Clone)]
pub struct NoteOutputNode {
    buffer: Buffer,
}

impl Node for NoteOutputNode {
    type State = NoteOutputNodeState;

    fn new() -> Self {
        Self {
            buffer: Buffer::default(),
        }
    }
    fn new_state(_creation_ctx: cubedaw_node::NodeCreationContext<'_>) -> Self::State {
        NoteOutputNodeState
    }

    fn process(&mut self, _state: &Self::State, ctx: &mut dyn NodeContext<'_>) {
        let buffer_size = ctx.buffer_size();
        self.buffer.resize(buffer_size);

        for i in 0..buffer_size {
            self.buffer[i as usize] = ctx.input(0).get(i);
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct NoteOutputNodeState;

impl NodeState for NoteOutputNodeState {
    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) {
        use cubedaw_lib::NodeInputUiOptions;

        ctx.output_ui(ui, "Track Input");
        ctx.input_ui(ui, "Note Output", NodeInputUiOptions::uninteractable());
    }
}
