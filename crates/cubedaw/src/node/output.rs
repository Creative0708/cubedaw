use cubedaw_lib::{Id, NodeState, NodeUiContext};
use cubedaw_node::{DynNode, Node, NodeContext};

use super::util::Buffer;

pub struct OutputNode {
    id: Id<DynNode>,
    buffer: Buffer,
}

impl Node for OutputNode {
    type State = OutputNodeUi;

    fn id(&self) -> cubedaw_lib::Id<DynNode> {
        self.id
    }
    fn spec(&self) -> (u32, u32) {
        (1, 0)
    }
    fn process(&mut self, ctx: &mut dyn NodeContext<'_>) {
        let buffer_size = ctx.buffer_size();

        let buf = self.buffer.resize_and_get_mut(buffer_size);
        for i in 0..buffer_size {
            buf[i as usize] = ctx.input(0).get(i);
        }
    }

    fn create_state(&self) -> Self::State {
        OutputNodeUi
    }
    fn update_from_state(&mut self, _: Self::State) {}
}

impl OutputNode {
    pub fn create(_ctx: super::NodeCreationContext) -> DynNode {
        Box::new(Self {
            id: Id::arbitrary(),
            buffer: Buffer::new(),
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct OutputNodeUi;

impl NodeState for OutputNodeUi {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) {
        ctx.input_ui(ui, "Output");
    }
}
