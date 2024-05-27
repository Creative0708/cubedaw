use cubedaw_lib::{Id, NodeState, NodeUiContext};
use cubedaw_node::{DynNode, Node, NodeContext};
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

impl Node for MathNode {
    type State = MathNodeUi;

    fn id(&self) -> Id<DynNode> {
        self.id
    }
    fn spec(&self) -> (u32, u32) {
        (2, 1)
    }
    fn process(&mut self, ctx: &mut dyn NodeContext<'_>) {
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

    fn create_state(&self) -> Self::State {
        Self::State {
            node_type: self.node_type,
        }
    }

    fn update_from_state(&mut self, data: Self::State) {
        self.node_type = data.node_type;
    }
}

impl MathNode {
    pub fn create(ctx: super::NodeCreationContext) -> Self {
        Self {
            id: Id::arbitrary(),
            node_type: match ctx.alias.as_deref() {
                Some("add") => MathNodeType::Add,
                Some("subtract") => MathNodeType::Subtract,
                Some("multiply") => MathNodeType::Multiply,
                Some("divide") => MathNodeType::Divide,

                _ => MathNodeType::Add,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathNodeUi {
    node_type: MathNodeType,
}

impl NodeState for MathNodeUi {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) {
        ComboBox::from_id_source(0)
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
        ctx.input_ui(ui, "A");
        ctx.input_ui(ui, "B");
        ctx.output_ui(ui, "Out");
    }
}
