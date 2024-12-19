use anyhow::Result;
use cubedaw_lib::Buffer;
use zerocopy::{IntoBytes, TryFromBytes};

use crate::{
    node::{NodeInputUiOptions, ui::PitchState},
    registry::NodeThingy,
};

mod schema;
pub use schema::*;

use super::ZerocopyTryFromExt;

impl OscillatorNodeType {
    const fn to_str(self) -> &'static str {
        match self {
            OscillatorNodeType::Sine => "Sine",
            OscillatorNodeType::Saw => "Saw",
            OscillatorNodeType::Square => "Square",
            OscillatorNodeType::Triangle => "Triangle",
        }
    }
    fn from_str(str: &str) -> Option<Self> {
        Some(match str {
            "sine" => Self::Sine,
            "saw" => Self::Saw,
            "square" => Self::Square,
            "triangle" => Self::Triangle,

            _ => return None,
        })
    }
}

pub struct OscillatorNode;

impl NodeThingy for OscillatorNode {
    fn create(&self, ctx: &crate::node::NodeCreationContext) -> Box<Buffer> {
        OscillatorNodeArgs {
            node_type: ctx
                .alias
                .as_deref()
                .and_then(OscillatorNodeType::from_str)
                .unwrap_or(OscillatorNodeType::Sine),
            pitch_state: PitchState::Relative,
            _pad1: Default::default(),
        }
        .as_bytes()
        .into()
    }
    fn title(&self, state_buf: &Buffer) -> Result<std::borrow::Cow<'_, str>> {
        let (state, _) = OscillatorNodeArgs::try_ref_from_prefix(state_buf.as_bytes()).anyhow()?;
        Ok(state.node_type.to_str().into())
    }
    fn ui(
        &self,
        state_buf: &mut Buffer,
        ui: &mut egui::Ui,
        ctx: &mut dyn crate::node::NodeUiContext,
    ) -> Result<()> {
        let (state, _) =
            OscillatorNodeArgs::try_mut_from_prefix(state_buf.as_bytes_mut()).anyhow()?;

        egui::ComboBox::from_id_salt(0)
            .selected_text(state.node_type.to_str())
            .show_ui(ui, |ui| {
                for ty in [
                    OscillatorNodeType::Sine,
                    OscillatorNodeType::Saw,
                    OscillatorNodeType::Square,
                    OscillatorNodeType::Triangle,
                ] {
                    ui.selectable_value(&mut state.node_type, ty, ty.to_str());
                }
            });

        // TODO implement plot

        ctx.input_ui(
            ui,
            "Pitch",
            NodeInputUiOptions::pitch_choice(&mut state.pitch_state),
        );
        ctx.output_ui(ui, "Out");

        Ok(())
    }

    fn make_nodefactory(&self) -> cubedaw_worker::DynNodeFactory {
        cubedaw_worker::DynNodeFactory::new_castable(|_| 0.0f32)
    }
}
