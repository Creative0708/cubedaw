use crate::BufferType;

pub struct TrackInputNode {
    buffer: Option<&'static [BufferType]>,
}

impl crate::Node for TrackInputNode {
    type State = TrackInputNodeState;

    fn new() -> Self {
        Self { buffer: None }
    }
    fn new_state(_creation_ctx: crate::NodeCreationContext<'_>) -> Self::State {
        TrackInputNodeState
    }

    fn process(&mut self, _state: &Self::State, ctx: &mut dyn crate::NodeContext<'_>) {
        let Some(ref mut buffer) = self.buffer else {
            panic!("self.buffer is None. WHO FORGOT TO SET THE FIELD RAAAAAHHHHHHH");
        };
        let buffer_size = ctx.buffer_size();
        debug_assert!(buffer.len() == buffer_size as usize);

        let output = ctx.output(0);

        for i in 0..buffer_size {
            output.set(i, buffer[i as usize]);
        }
    }
}

impl TrackInputNode {
    pub fn start(&mut self, buffer: &'static [BufferType]) {
        let old = self.buffer.replace(buffer);
        #[cfg(debug_assertions)]
        if old.is_some() {
            panic!("start() called on TrackInputNode with a buffer")
        }
    }
    pub fn end(&mut self) -> &'static [BufferType] {
        self.buffer
            .take()
            .expect("end() called on TrackInputNode without a buffer")
    }
}

impl Clone for TrackInputNode {
    fn clone(&self) -> Self {
        if self.buffer.is_some() {
            panic!("clone() called on live TrackInputNode");
        }

        Self { buffer: None }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TrackInputNodeState;

impl crate::NodeState for TrackInputNodeState {
    fn title(&self) -> std::borrow::Cow<'static, str> {
        "Track Input".into()
    }

    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn crate::NodeUiContext) {
        ctx.input_ui(ui, "Input", crate::NodeInputUiOptions::uninteractable());
    }
}
