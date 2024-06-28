#[derive(Clone)]
pub struct NoteOutputNode {
    buffer: crate::Buffer,
}

impl crate::Node for NoteOutputNode {
    type State = NoteOutputNodeState;

    fn new() -> Self {
        Self {
            buffer: crate::Buffer::default(),
        }
    }
    fn new_state(_creation_ctx: crate::NodeCreationContext<'_>) -> Self::State {
        NoteOutputNodeState
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
pub struct NoteOutputNodeState;

impl crate::NodeState for NoteOutputNodeState {
    fn title(&self) -> std::borrow::Cow<'static, str> {
        "Node Output".into()
    }

    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn crate::NodeUiContext) {
        ctx.output_ui(ui, "Track Input");
        ctx.input_ui(
            ui,
            "Note Output",
            crate::NodeInputUiOptions::uninteractable(),
        );
    }
}
