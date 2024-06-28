#[derive(Clone)]
pub struct TrackOutputNode {
    buffer: crate::Buffer,
}

impl crate::Node for TrackOutputNode {
    type State = TrackOutputNodeState;

    fn new() -> Self {
        Self {
            buffer: crate::Buffer::default(),
        }
    }
    fn new_state(_creation_ctx: crate::NodeCreationContext<'_>) -> Self::State {
        TrackOutputNodeState
    }

    fn process(&mut self, _state: &Self::State, ctx: &mut dyn crate::NodeContext<'_>) {
        let buffer_size = ctx.buffer_size();
        self.buffer.resize(buffer_size);

        let input = ctx.input(0);

        for i in 0..buffer_size {
            self.buffer[i as usize] = input[i];
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TrackOutputNodeState;

impl crate::NodeState for TrackOutputNodeState {
    fn title(&self) -> std::borrow::Cow<'static, str> {
        "Track Output".into()
    }

    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn crate::NodeUiContext) {
        ctx.input_ui(ui, "Output", crate::NodeInputUiOptions::uninteractable());
    }
}
