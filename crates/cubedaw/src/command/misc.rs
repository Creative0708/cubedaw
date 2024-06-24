use super::UiStateCommand;

pub struct UiSetPlayhead {
    old_pos: f32,
    new_pos: f32,
}

impl UiSetPlayhead {
    pub fn new(pos: f32) -> Self {
        Self {
            old_pos: f32::NAN,
            new_pos: pos,
        }
    }
}

impl UiStateCommand for UiSetPlayhead {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        self.old_pos = ui_state.playhead_pos;
        ui_state.playhead_pos = self.new_pos;
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        ui_state.playhead_pos = self.old_pos;
    }

    fn try_merge(&mut self, other: &Self) -> bool {
        self.new_pos = other.new_pos;
        true
    }
}
