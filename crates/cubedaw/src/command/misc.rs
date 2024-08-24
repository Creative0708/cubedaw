use super::UiStateCommand;

pub struct UiSetPlayhead {
    old_pos: i64,
    new_pos: i64,
}

impl UiSetPlayhead {
    pub fn new(pos: i64) -> Self {
        Self {
            old_pos: 0,
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
