use anyhow::Result;
use cubedaw_worker::DynNodeFactory;

use crate::registry::NodeThingy;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
enum MathNodeType {
    Add = 0,
    Subtract = 1,
    Multiply = 2,
    Divide = 3,
}

impl MathNodeType {
    fn from_arr(arr: &[u8]) -> Result<Self> {
        let arr: [u8; 4] = arr.try_into()?;

        Ok(match u32::from_le_bytes(arr) {
            0 => Self::Add,
            1 => Self::Subtract,
            2 => Self::Multiply,
            3 => Self::Divide,
            other => anyhow::bail!("invalid index for MathNodeType: {other}"),
        })
    }
    fn to_arr(&self) -> Box<[u8]> {
        (*self as u32).to_le_bytes().into()
    }

    const fn to_str(&self) -> &'static str {
        match self {
            MathNodeType::Add => "Add",
            MathNodeType::Subtract => "Subtract",
            MathNodeType::Multiply => "Multiply",
            MathNodeType::Divide => "Divide",
        }
    }
}

pub struct MathNode;

impl NodeThingy for MathNode {
    fn create(&self, ctx: &crate::node::NodeCreationContext) -> Box<[u8]> {
        let node_type = match ctx.alias.as_deref() {
            Some("add") => MathNodeType::Add,
            Some("subtract") => MathNodeType::Subtract,
            Some("multiply") => MathNodeType::Multiply,
            Some("divide") => MathNodeType::Divide,

            _ => MathNodeType::Add,
        };
        node_type.to_arr()
    }
    fn title(&self, state: &[u8]) -> Result<std::borrow::Cow<'_, str>> {
        let node_type = MathNodeType::from_arr(state)?;
        Ok(node_type.to_str().into())
    }
    fn ui(
        &self,
        state: &mut [u8],
        ui: &mut egui::Ui,
        ctx: &mut dyn crate::node::NodeUiContext,
    ) -> Result<()> {
        let mut node_type = MathNodeType::from_arr(state)?;

        ctx.output_ui(ui, "Out");

        egui::ComboBox::from_id_salt(0)
            .selected_text(node_type.to_str())
            .show_ui(ui, |ui| {
                for ty in [
                    MathNodeType::Add,
                    MathNodeType::Subtract,
                    MathNodeType::Multiply,
                    MathNodeType::Divide,
                ] {
                    ui.selectable_value(&mut node_type, ty, ty.to_str());
                }
            });

        // TODO implement plot

        ctx.input_ui(ui, "A", Default::default());
        ctx.input_ui(ui, "B", Default::default());
        ctx.output_ui(ui, "Out");

        state.copy_from_slice(&node_type.to_arr());

        Ok(())
    }
}
