use crate::provider::{EditProviderInput, NewProviderInput};

pub enum UiAction {
    Continue,
    Quit,
    RunSelected(String),
    ExecSelected(String),
    SubmitAdd(NewProviderInput),
    SubmitEdit {
        original_name: String,
        input: EditProviderInput,
    },
    DeleteSelected(String),
}
