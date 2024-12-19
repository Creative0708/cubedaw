use anyhow::Result;
use cubedaw_lib::Buffer;
use zerocopy::{IntoBytes, TryFromBytes};

use crate::registry::NodeThingy;

use super::ZerocopyTryFromExt;

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Debug,
    zerocopy::TryFromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(u32)]
enum MathNodeType {
    Add = 0,
    Subtract = 1,
    Multiply = 2,
    Divide = 3,
}

impl MathNodeType {
    const fn to_str(&self) -> &'static str {
        match self {
            MathNodeType::Add => "Add",
            MathNodeType::Subtract => "Subtract",
            MathNodeType::Multiply => "Multiply",
            MathNodeType::Divide => "Divide",
        }
    }
}

#[repr(C)]
#[derive(
    zerocopy::TryFromBytes, zerocopy::IntoBytes, zerocopy::Immutable, zerocopy::KnownLayout,
)]
struct MathNodeState {
    node_type: MathNodeType,
}

pub struct MathNode;

impl NodeThingy for MathNode {
    fn create(&self, ctx: &crate::node::NodeCreationContext) -> Box<Buffer> {
        let node_type = match ctx.alias.as_deref() {
            Some("add") => MathNodeType::Add,
            Some("subtract") => MathNodeType::Subtract,
            Some("multiply") => MathNodeType::Multiply,
            Some("divide") => MathNodeType::Divide,

            _ => MathNodeType::Add,
        };
        MathNodeState { node_type }.as_bytes().into()
    }
    fn title(&self, state: &Buffer) -> Result<std::borrow::Cow<'_, str>> {
        let (node_state, _) = MathNodeState::try_ref_from_prefix(state.as_bytes()).anyhow()?;
        Ok(node_state.node_type.to_str().into())
    }
    fn ui(
        &self,
        state: &mut Buffer,
        ui: &mut egui::Ui,
        ctx: &mut dyn crate::node::NodeUiContext,
    ) -> Result<()> {
        let (node_state, _) = MathNodeState::try_mut_from_prefix(state.as_bytes_mut()).anyhow()?;

        ctx.output_ui(ui, "Out");

        egui::ComboBox::from_id_salt(0)
            .selected_text(node_state.node_type.to_str())
            .show_ui(ui, |ui| {
                for ty in [
                    MathNodeType::Add,
                    MathNodeType::Subtract,
                    MathNodeType::Multiply,
                    MathNodeType::Divide,
                ] {
                    ui.selectable_value(&mut node_state.node_type, ty, ty.to_str());
                }
            });

        // TODO implement plot

        ctx.input_ui(ui, "A", Default::default());
        ctx.input_ui(ui, "B", Default::default());
        ctx.output_ui(ui, "Out");

        Ok(())
    }
}
