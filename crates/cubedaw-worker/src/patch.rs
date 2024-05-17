use std::any::Any;

use cubedaw_lib::Id;

pub type DynNode = Box<dyn NodeWrapper>;

pub enum DataSource<'a> {
    Const(f32),
    NodeOutput(&'a [f32]),
}
impl<'a> DataSource<'a> {
    pub fn get(&self, i: u32) -> f32 {
        match self {
            Self::Const(val) => *val,
            Self::NodeOutput(buf) => buf[i as usize],
        }
    }
}
pub enum DataDrain<'a> {
    Disconnected,
    NodeInput(&'a mut [f32]),
}
impl<'a> DataDrain<'a> {
    pub fn set(&mut self, i: u32, val: f32) {
        match self {
            Self::Disconnected => (),
            Self::NodeInput(buf) => {
                buf[i as usize] = val;
            }
        }
    }
}

pub trait NodeContext<'a> {
    fn sample_rate(&self) -> u32;
    fn buffer_size(&self) -> u32;
    fn input(&self, index: u32) -> DataSource<'a>;
    fn output(&self, index: u32) -> DataDrain<'a>;
}

#[cfg(feature = "egui")]
pub trait PatchContext<'a> {
    fn input_ui(&mut self, ui: &mut egui::Ui, name: &str);
    fn output_ui(&mut self, ui: &mut egui::Ui, name: &str);
}

pub trait Node: Send {
    // the reason for the whole Self::Ui thing is to have a way to have the ui thread render without waiting for thread synchronization
    // (which could cause very bad ui delays.)
    type Ui: NodeUi;

    fn id(&self) -> Id<DynNode>;
    // -> (# of inputs, # of outputs)
    fn spec(&self) -> (u32, u32);

    fn process(&mut self, ctx: &mut dyn NodeContext<'_>);

    /// Creates a `Self::Ui` representing the current state
    fn create_ui(&self) -> Self::Ui;
    fn update_from_ui(&mut self, data: Self::Ui);
}

pub trait NodeUi: 'static + Sized + Send + Clone {
    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, patch_ctx: &mut dyn PatchContext<'_>);
}

// unsafe because the returned Box<dyn Any> from default_data has to be the associated NodeUi
pub unsafe trait NodeWrapper: Send {
    fn id(&self) -> Id<DynNode>;
    fn spec(&self) -> (u32, u32);
    fn process(&mut self, ctx: &mut dyn NodeContext<'_>);

    fn create_ui(&self) -> *mut ();

    // SAFETY: data has to be the same type as the return of create_ui
    unsafe fn update_from_ui(&mut self, data: *mut ());
}

unsafe impl<T: Node> NodeWrapper for T {
    fn id(&self) -> Id<DynNode> {
        self.id()
    }

    fn spec(&self) -> (u32, u32) {
        self.spec()
    }

    fn process(&mut self, ctx: &mut dyn NodeContext<'_>) {
        self.process(ctx)
    }

    fn create_ui(&self) -> *mut () {
        Box::leak(Box::new(self.create_ui())) as *mut T::Ui as *mut ()
    }

    unsafe fn update_from_ui(&mut self, data: *mut ()) {
        self.update_from_ui(unsafe { *Box::from_raw(data as *mut T::Ui) })
    }
}
