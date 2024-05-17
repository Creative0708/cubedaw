use cubedaw_lib::Id;
use cubedaw_worker::patch::{DynNode, NodeUi};

use super::util::Buffer;

pub struct OutputNode {
    id: Id<DynNode>,
    buffer: Buffer,
}

impl cubedaw_worker::patch::Node for OutputNode {
    type Ui = OutputNodeUi;

    fn id(&self) -> cubedaw_lib::Id<DynNode> {
        self.id
    }
    fn spec(&self) -> (u32, u32) {
        (1, 0)
    }
    fn process(&mut self, ctx: &mut dyn cubedaw_worker::patch::NodeContext<'_>) {
        let buffer_size = ctx.buffer_size();

        let buf = self.buffer.resize_and_get_mut(buffer_size);
        for i in 0..buffer_size {
            buf[i as usize] = ctx.input(0).get(i);
        }
    }

    fn create_ui(&self) -> Self::Ui {
        OutputNodeUi
    }
    fn update_from_ui(&mut self, _: Self::Ui) {}
}

impl OutputNode {
    pub fn create(_ctx: super::NodeCreationContext) -> DynNode {
        Box::new(Self {
            id: Id::arbitrary(),
            buffer: Buffer::new(),
        })
    }
}

#[derive(Clone)]
pub struct OutputNodeUi;

impl NodeUi for OutputNodeUi {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        patch_ctx: &mut dyn cubedaw_worker::patch::PatchContext<'_>,
    ) {
        patch_ctx.input_ui(ui, "Output");
    }
}
