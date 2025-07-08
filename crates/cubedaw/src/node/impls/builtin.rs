use std::{borrow::Cow, num::NonZero};

use anyhow::{Context as _, Result};
use cubedaw_lib::{Buffer, Id, Track};

use crate::{
    Context,
    node::{NodeCreationContext, NodeInputUiOptions, NodeUiContext},
    registry::NodeUi,
};

pub struct TrackInputNodeUi;
impl NodeUi for TrackInputNodeUi {
    fn create(&self, _creation_context: &NodeCreationContext) -> Box<Buffer> {
        Default::default()
    }
    fn title(&self, buf: &Buffer, ctx: &Context) -> Result<Cow<'_, str>> {
        let track_id = Id::from_raw(
            NonZero::new(u64::from_le_bytes(buf.as_bytes().try_into()?))
                .context("null track id")?,
        );
        Ok(match ctx.ui_state.tracks.get(track_id) {
            Some(track) => Cow::Owned(track.name.clone()),
            None => Cow::Borrowed("<invalid track>"),
        })
    }
    fn ui(&self, _: &mut Buffer, ui: &mut egui::Ui, node_ui: &mut dyn NodeUiContext) -> Result<()> {
        node_ui.output_ui(ui, "Track Input");
        Ok(())
    }

    fn make_node_factory(&self) -> cubedaw_worker::DynNodeFactory {
        unreachable!("builtin nodes don't have node factories");
    }
}

pub struct OutputNodeUi;
impl NodeUi for OutputNodeUi {
    fn create(&self, _creation_context: &NodeCreationContext) -> Box<Buffer> {
        Default::default()
    }
    fn title(&self, _: &Buffer, _ctx: &Context) -> Result<std::borrow::Cow<'_, str>> {
        Ok("Output".into())
    }
    fn ui(&self, _: &mut Buffer, ui: &mut egui::Ui, node_ui: &mut dyn NodeUiContext) -> Result<()> {
        node_ui.input_ui(ui, "Output", NodeInputUiOptions::uninteractable());
        Ok(())
    }

    fn make_node_factory(&self) -> cubedaw_worker::DynNodeFactory {
        unreachable!("builtin nodes don't have node factories");
    }
}

pub struct DownmixNodeUi;
impl NodeUi for DownmixNodeUi {
    fn create(&self, _creation_context: &NodeCreationContext) -> Box<Buffer> {
        Default::default()
    }
    fn title(&self, _: &Buffer, _ctx: &Context) -> Result<std::borrow::Cow<'_, str>> {
        Ok("Downmix".into())
    }
    fn ui(&self, _: &mut Buffer, ui: &mut egui::Ui, node_ui: &mut dyn NodeUiContext) -> Result<()> {
        node_ui.input_ui(ui, "Input", NodeInputUiOptions::uninteractable());
        node_ui.output_ui(ui, "Output");
        Ok(())
    }

    fn make_node_factory(&self) -> cubedaw_worker::DynNodeFactory {
        unreachable!("builtin nodes don't have node factories");
    }
}
