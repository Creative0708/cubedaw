use cubedaw_lib::Id;
use cubedaw_worker::patch::{DynNode, NodeUi};
use egui::ComboBox;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MathNodeType {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl MathNodeType {
    const fn to_str(&self) -> &'static str {
        match self {
            MathNodeType::Add => "add",
            MathNodeType::Subtract => "subtract",
            MathNodeType::Multiply => "multiply",
            MathNodeType::Divide => "divide",
        }
    }
}

pub struct MathNode {
    id: Id<DynNode>,
    node_type: MathNodeType,
}

impl cubedaw_worker::patch::Node for MathNode {
    type Ui = MathNodeUi;

    fn id(&self) -> Id<DynNode> {
        self.id
    }
    fn spec(&self) -> (u32, u32) {
        (2, 1)
    }
    fn process(&mut self, ctx: &mut dyn cubedaw_worker::patch::NodeContext<'_>) {
        let a_in = ctx.input(0);
        let b_in = ctx.input(1);
        let mut out = ctx.output(0);

        for i in 0..ctx.buffer_size() {
            let a = a_in.get(i);
            let b = b_in.get(i);
            out.set(
                i,
                match self.node_type {
                    MathNodeType::Add => a + b,
                    MathNodeType::Subtract => a - b,
                    MathNodeType::Multiply => a * b,
                    MathNodeType::Divide => a / b,
                },
            )
        }
    }

    fn create_ui(&self) -> Self::Ui {
        Self::Ui {
            id: self.id,
            node_type: self.node_type,
        }
    }

    fn update_from_ui(&mut self, data: Self::Ui) {
        self.node_type = data.node_type;
    }
}

impl MathNode {
    pub fn create(ctx: super::NodeCreationContext) -> DynNode {
        Box::new(Self {
            id: Id::arbitrary(),
            node_type: match ctx.alias {
                Some("add") => MathNodeType::Add,
                Some("subtract") => MathNodeType::Subtract,
                Some("multiply") => MathNodeType::Multiply,
                Some("divide") => MathNodeType::Divide,

                _ => MathNodeType::Add,
            },
        })
    }
}

#[derive(Clone, Debug)]
pub struct MathNodeUi {
    id: Id<DynNode>,
    node_type: MathNodeType,
}

impl NodeUi for MathNodeUi {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        patch_ctx: &mut dyn cubedaw_worker::patch::PatchContext<'_>,
    ) {
        ComboBox::from_id_source(self.id)
            .selected_text(self.node_type.to_str())
            .show_ui(ui, |ui| {
                for node_type in [
                    MathNodeType::Add,
                    MathNodeType::Subtract,
                    MathNodeType::Multiply,
                    MathNodeType::Divide,
                ] {
                    ui.selectable_value(&mut self.node_type, node_type, node_type.to_str());
                }
            });
        patch_ctx.input_ui(ui, "A");
        patch_ctx.input_ui(ui, "B");
        patch_ctx.output_ui(ui, "Out");
    }
}
