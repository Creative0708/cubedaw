use anyhow::Result;

use crate::{node::NodeInputUiOptions, registry::NodeThingy};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum OscillatorNodeType {
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
}
impl OscillatorNodeType {
    fn from_arr(arr: &[u8]) -> Result<Self> {
        let arr: [u8; 4] = arr.try_into()?;

        Ok(match u32::from_le_bytes(arr) {
            0 => Self::Sine,
            1 => Self::Saw,
            2 => Self::Square,
            3 => Self::Triangle,
            other => anyhow::bail!("invalid index for MathNodeType: {other}"),
        })
    }
    fn to_arr(&self) -> Box<[u8]> {
        (*self as u32).to_le_bytes().into()
    }

    const fn to_str(&self) -> &'static str {
        match self {
            OscillatorNodeType::Sine => "Sine",
            OscillatorNodeType::Saw => "Saw",
            OscillatorNodeType::Square => "Square",
            OscillatorNodeType::Triangle => "Triangle",
        }
    }
}

pub struct OscillatorNode;

impl NodeThingy for OscillatorNode {
    fn create(&self, ctx: &crate::node::NodeCreationContext) -> Box<[u8]> {
        let node_type = match ctx.alias.as_deref() {
            Some("sine") => OscillatorNodeType::Sine,
            Some("saw") => OscillatorNodeType::Saw,
            Some("square") => OscillatorNodeType::Square,
            Some("triangle") => OscillatorNodeType::Triangle,

            _ => OscillatorNodeType::Sine,
        };
        node_type.to_arr()
    }
    fn title(&self, state: &[u8]) -> anyhow::Result<std::borrow::Cow<'_, str>> {
        let node_type = OscillatorNodeType::from_arr(state)?;
        Ok(node_type.to_str().into())
    }
    fn ui(
        &self,
        state: &mut [u8],
        ui: &mut egui::Ui,
        ctx: &mut dyn crate::node::NodeUiContext,
    ) -> anyhow::Result<()> {
        let mut node_type = OscillatorNodeType::from_arr(state)?;

        egui::ComboBox::from_id_salt(0)
            .selected_text(node_type.to_str())
            .show_ui(ui, |ui| {
                for ty in [
                    OscillatorNodeType::Sine,
                    OscillatorNodeType::Saw,
                    OscillatorNodeType::Square,
                    OscillatorNodeType::Triangle,
                ] {
                    ui.selectable_value(&mut node_type, ty, ty.to_str());
                }
            });

        // TODO implement plot

        ctx.input_ui(ui, "Pitch", NodeInputUiOptions::pitch());
        ctx.output_ui(ui, "Out");

        state.copy_from_slice(&node_type.to_arr());

        Ok(())
    }
}
