// An `egui::Label` but turns into a `egui::TextEdit` when double-clicked.
pub struct EditableLabel<'a, F: FnOnce(&str) -> egui::RichText = fn(&str) -> egui::RichText> {
    string: &'a mut String,

    id: Option<egui::Id>,
    id_salt: Option<egui::Id>,

    formatter: F,
}

impl<'a, F: FnOnce(&str) -> egui::RichText> EditableLabel<'a, F> {
    pub fn with_formatter(string: &'a mut String, formatter: F) -> Self {
        Self {
            string,

            id: None,
            id_salt: None,

            formatter,
        }
    }

    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn id_salt(mut self, id_salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(egui::Id::new(id_salt));
        self
    }
}

impl<'a> EditableLabel<'a> {
    pub fn new(string: &'a mut String) -> Self {
        Self::with_formatter(string, |str| str.into())
    }
}

impl<'a, F: FnOnce(&str) -> egui::RichText> egui::Widget for EditableLabel<'a, F> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            string,
            id,
            id_salt,
            formatter,
        } = self;

        let auto_id = ui.next_auto_id();

        let id = id.unwrap_or(match id_salt {
            Some(id_salt) => ui.make_persistent_id(id_salt),
            None => auto_id,
        });

        let state: EditableLabelState = EditableLabelState::load(ui.ctx(), id).unwrap_or_default();

        let response;

        let state = match state {
            EditableLabelState::NotEditing => {
                response = ui.add(egui::Label::new(formatter(string)));

                if response.double_clicked() {
                    let mut text_edit_state = egui::text_edit::TextEditState::default();
                    text_edit_state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(string.len()),
                        )));
                    text_edit_state.store(ui.ctx(), id);

                    EditableLabelState::Editing(string.clone())
                } else {
                    state
                }
            }
            EditableLabelState::Editing(mut curr_string) => {
                response = ui.add(egui::TextEdit::singleline(&mut curr_string).id(id));

                if response.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        // user cancelled the edit
                        EditableLabelState::NotEditing
                    } else {
                        *string = curr_string;

                        EditableLabelState::NotEditing
                    }
                } else {
                    if !response.has_focus() {
                        response.request_focus();
                    }

                    EditableLabelState::Editing(curr_string)
                }
            }
        };

        state.store(ui.ctx(), id);

        response
    }
}

#[derive(Clone, Default)]
pub enum EditableLabelState {
    #[default]
    NotEditing,
    Editing(String),
}
impl EditableLabelState {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Option<Self> {
        ctx.data(|d| d.get_temp(id))
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_temp(id, self))
    }
}
