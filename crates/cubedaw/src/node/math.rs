use cubedaw_lib::{Node, NodeContext};
use cubedaw_lib::{NodeState, NodeUiContext};
use egui::ComboBox;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MathNodeType {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl MathNodeType {
    const fn to_str(self) -> &'static str {
        match self {
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Multiply => "Multiply",
            Self::Divide => "Divide",
        }
    }
}

#[derive(Clone)]
pub struct MathNode;

impl Node for MathNode {
    type State = MathNodeState;

    fn new() -> Self {
        Self
    }

    fn new_state(ctx: cubedaw_lib::NodeCreationContext) -> Self::State {
        Self::State {
            node_type: match ctx.alias.as_deref() {
                Some("add") => MathNodeType::Add,
                Some("subtract") => MathNodeType::Subtract,
                Some("multiply") => MathNodeType::Multiply,
                Some("divide") => MathNodeType::Divide,

                _ => MathNodeType::Add,
            },
        }
    }

    // TODO optimize
    fn process(&mut self, state: &Self::State, ctx: &mut dyn NodeContext<'_>) {
        let a_in = ctx.input(0);
        let b_in = ctx.input(1);
        let out = ctx.output(0);

        for i in 0..ctx.buffer_size() {
            let a = a_in[i];
            let b = b_in[i];
            out.set(
                i,
                match state.node_type {
                    MathNodeType::Add => a + b,
                    MathNodeType::Subtract => a - b,
                    MathNodeType::Multiply => a * b,
                    MathNodeType::Divide => a / b,
                },
            )
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathNodeState {
    node_type: MathNodeType,
}

impl NodeState for MathNodeState {
    fn title(&self) -> std::borrow::Cow<'_, str> {
        self.node_type.to_str().into()
    }

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
        ctx.input_ui(ui, "A", Default::default());
        ctx.input_ui(ui, "B", Default::default());
        ctx.output_ui(ui, "Out");
    }
}
