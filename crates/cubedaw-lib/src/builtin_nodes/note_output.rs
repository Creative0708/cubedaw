use crate::BufferType;

pub struct NoteOutputNode {
    inner: Option<NoteOutputNodeInner>,
}

pub enum NoteOutputNodeInner {
    Input(&'static [BufferType]),
    Output(&'static mut [BufferType]),
}

impl crate::Node for NoteOutputNode {
    type State = NoteOutputNodeState;

    fn new() -> Self {
        Self { inner: None }
    }
    fn new_state(_creation_ctx: crate::NodeCreationContext<'_>) -> Self::State {
        NoteOutputNodeState
    }

    fn process(&mut self, _state: &Self::State, ctx: &mut dyn crate::NodeContext<'_>) {
        let buffer_size = ctx.buffer_size();
        match self.inner {
            None => panic!("process() called without setting buffer"),
            Some(NoteOutputNodeInner::Input(buffer)) => {
                debug_assert!(buffer.len() == buffer_size as usize);
                let output = ctx.output(0);
                for i in 0..buffer_size {
                    output.set(i, buffer[i as usize]);
                }
            }
            Some(NoteOutputNodeInner::Output(ref mut buffer)) => {
                let input = ctx.input(0);
                for i in 0..buffer_size {
                    buffer[i as usize] = input[i];
                }
            }
        }
    }
}

impl NoteOutputNode {
    pub fn start(&mut self, inner: NoteOutputNodeInner) {
        if self.inner.replace(inner).is_some() {
            panic!("set_buffer() called on TrackOutputNode with a buffer")
        }
    }
    pub fn end(&mut self) -> NoteOutputNodeInner {
        self.inner
            .take()
            .expect("take_buffer() called on TrackOutputNode without a buffer")
    }
}

impl Clone for NoteOutputNode {
    fn clone(&self) -> Self {
        if self.inner.is_some() {
            panic!("clone() called on live NoteOutputNode");
        }

        Self { inner: None }
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
