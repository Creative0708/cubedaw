pub struct TrackOutputNode {
    buffer: Option<crate::BufferOwned>,
}

impl crate::Node for TrackOutputNode {
    type State = TrackOutputNodeState;

    fn new() -> Self {
        Self { buffer: None }
    }
    fn new_state(_creation_ctx: crate::NodeCreationContext<'_>) -> Self::State {
        TrackOutputNodeState
    }

    fn process(&mut self, _state: &Self::State, ctx: &mut dyn crate::NodeContext<'_>) {
        let Some(ref mut buffer) = self.buffer else {
            panic!("self.buffer is None. WHO FORGOT TO SET THE FIELD RAAAAAHHHHHHH");
        };
        let buffer_size = ctx.buffer_size();
        debug_assert_eq!(buffer.len(), buffer_size, "buffer size mismatch");

        let input = ctx.input(0);

        for i in 0..buffer_size {
            buffer[i] = input[i];
        }
    }
}

impl TrackOutputNode {
    pub fn start(&mut self, buffer: crate::BufferOwned) {
        let old = self.buffer.replace(buffer);
        #[cfg(debug_assertions)]
        if old.is_some() {
            panic!("set_buffer() called on TrackOutputNode with a buffer")
        }
    }
    pub fn end(&mut self) -> crate::BufferOwned {
        self.buffer
            .take()
            .expect("take_buffer() called on TrackOutputNode without a buffer")
    }
}

impl Clone for TrackOutputNode {
    fn clone(&self) -> Self {
        if self.buffer.is_some() {
            panic!("clone() called on live TrackOutputNode");
        }

        Self { buffer: None }
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
