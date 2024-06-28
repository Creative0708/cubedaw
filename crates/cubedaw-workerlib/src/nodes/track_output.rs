use cubedaw_lib::{NodeState, NodeUiContext};
use cubedaw_node::{Node, NodeContext};

use crate::Buffer;

#[derive(Clone)]
pub struct TrackOutputNode {
    buffer: Buffer,
}

impl Node for TrackOutputNode {
    type State = TrackOutputNodeState;

    fn new() -> Self {
        Self {
            buffer: Buffer::default(),
        }
    }
    fn new_state(_creation_ctx: cubedaw_node::NodeCreationContext<'_>) -> Self::State {
        TrackOutputNodeState
    }

    fn process(&mut self, _state: &Self::State, ctx: &mut dyn NodeContext<'_>) {
        let buffer_size = ctx.buffer_size();
        self.buffer.resize(buffer_size);

        let input = ctx.input(0);

        for i in 0..buffer_size {
            self.buffer[i as usize] = input[i];
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TrackOutputNodeState;

impl NodeState for TrackOutputNodeState {
    fn title(&self) -> std::borrow::Cow<'static, str> {
        "Track Output".into()
    }

    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) {
        use cubedaw_lib::NodeInputUiOptions;

        ctx.input_ui(ui, "Output", NodeInputUiOptions::uninteractable());
    }
}
