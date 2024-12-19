use anyhow::Result;
use cubedaw_lib::Buffer;

use crate::{
    node::{NodeCreationContext, NodeInputUiOptions, NodeUiContext},
    registry::NodeThingy,
};

pub struct TrackInputNodeThingy;
impl NodeThingy for TrackInputNodeThingy {
    fn create(&self, _creation_context: &NodeCreationContext) -> Box<Buffer> {
        Default::default()
    }
    fn title(&self, _: &Buffer) -> Result<std::borrow::Cow<'_, str>> {
        Ok("Track Input".into())
    }
    fn ui(&self, _: &mut Buffer, ui: &mut egui::Ui, node_ui: &mut dyn NodeUiContext) -> Result<()> {
        node_ui.output_ui(ui, "Track Input");
        Ok(())
    }

    fn make_nodefactory(&self) -> cubedaw_worker::DynNodeFactory {
        unreachable!("builtin nodes don't have node factories");
    }
}

pub struct TrackOutputNodeThingy;
impl NodeThingy for TrackOutputNodeThingy {
    fn create(&self, _creation_context: &NodeCreationContext) -> Box<Buffer> {
        Default::default()
    }
    fn title(&self, _: &Buffer) -> Result<std::borrow::Cow<'_, str>> {
        Ok("Track Output".into())
    }
    fn ui(&self, _: &mut Buffer, ui: &mut egui::Ui, node_ui: &mut dyn NodeUiContext) -> Result<()> {
        node_ui.input_ui(ui, "Track Output", NodeInputUiOptions::uninteractable());
        Ok(())
    }

    fn make_nodefactory(&self) -> cubedaw_worker::DynNodeFactory {
        unreachable!("builtin nodes don't have node factories");
    }
}

pub struct NoteInputNodeThingy;
impl NodeThingy for NoteInputNodeThingy {
    fn create(&self, _creation_context: &NodeCreationContext) -> Box<Buffer> {
        Default::default()
    }
    fn title(&self, _: &Buffer) -> Result<std::borrow::Cow<'_, str>> {
        Ok("Note Input".into())
    }
    fn ui(&self, _: &mut Buffer, ui: &mut egui::Ui, node_ui: &mut dyn NodeUiContext) -> Result<()> {
        node_ui.input_ui(ui, "Note Output", NodeInputUiOptions::uninteractable());
        node_ui.output_ui(ui, "Track Input");
        Ok(())
    }

    fn make_nodefactory(&self) -> cubedaw_worker::DynNodeFactory {
        unreachable!("builtin nodes don't have node factories");
    }
}
