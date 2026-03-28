use crate::provider_templates::{TemplateChoice, template_choices};
use crate::ui::action::UiAction;
use crate::ui::state::{
    AddFormState, AddTemplateState, ListScreenState, SearchState, UiMode, UiScreen, UiState,
    provider_matches_search, template_matches_search,
};
use crossterm::event::KeyCode;

pub fn handle_key(state: &mut UiState, code: KeyCode) -> UiAction {
    match &state.screen {
        UiScreen::List(_) => handle_list_key(state, code),
        UiScreen::AddTemplate(_) => handle_add_template_key(state, code),
        UiScreen::AddForm(_) => handle_add_form_key(state, code),
        UiScreen::EditForm(_) => handle_edit_form_key(state, code),
        UiScreen::DeleteConfirm => handle_delete_confirm_key(state, code),
    }
}

fn handle_list_key(state: &mut UiState, code: KeyCode) -> UiAction {
    if list_search_active(state) {
        return handle_list_search_key(state, code);
    }

    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            move_provider_selection(state, 1);
            UiAction::Continue
        }
        KeyCode::Up | KeyCode::Char('k') => {
            move_provider_selection(state, -1);
            UiAction::Continue
        }
        KeyCode::Enter => selected_provider_name(state)
            .map(|name| match state.mode {
                UiMode::Exec => UiAction::ExecSelected(name),
                UiMode::Manage | UiMode::Run => UiAction::RunSelected(name),
            })
            .unwrap_or(UiAction::Continue),
        KeyCode::Char('/') => {
            if let UiScreen::List(list) = &mut state.screen {
                list.search.active = true;
            }
            UiAction::Continue
        }
        KeyCode::Char('n') => {
            state.screen = UiScreen::AddTemplate(AddTemplateState::new(default_template_choices()));
            UiAction::Continue
        }
        KeyCode::Char('e') => selected_provider_name(state)
            .map(|name| {
                let selected = state
                    .providers
                    .iter()
                    .position(|provider| provider.name == name)
                    .unwrap();
                state.screen = UiScreen::EditForm(crate::ui::state::EditFormState::from_provider(
                    &state.providers[selected],
                ));
                UiAction::Continue
            })
            .unwrap_or(UiAction::Continue),
        KeyCode::Char('d') => selected_provider_name(state)
            .map(|_| {
                state.screen = UiScreen::DeleteConfirm;
                UiAction::Continue
            })
            .unwrap_or(UiAction::Continue),
        KeyCode::Esc | KeyCode::Char('q') => UiAction::Quit,
        _ => UiAction::Continue,
    }
}

fn handle_add_template_key(state: &mut UiState, code: KeyCode) -> UiAction {
    if template_search_active(state) {
        return handle_template_search_key(state, code);
    }

    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            move_template_selection(state, 1);
            UiAction::Continue
        }
        KeyCode::Up | KeyCode::Char('k') => {
            move_template_selection(state, -1);
            UiAction::Continue
        }
        KeyCode::Char('/') => {
            if let UiScreen::AddTemplate(add_template) = &mut state.screen {
                add_template.search.active = true;
            }
            UiAction::Continue
        }
        KeyCode::Enter => {
            let Some(selected) = selected_template_choice(state) else {
                return UiAction::Continue;
            };
            state.screen = UiScreen::AddForm(match selected {
                TemplateChoice::BuiltIn(template) => AddFormState {
                    active_field: 2,
                    name: template.id,
                    base_url: template.base_url,
                    api_key: String::new(),
                    model: template.default_model.unwrap_or_default(),
                    notes: template.notes.unwrap_or_default(),
                    error: None,
                },
                TemplateChoice::Custom => AddFormState::custom(),
            });
            UiAction::Continue
        }
        KeyCode::Esc => {
            state.screen = list_screen();
            UiAction::Continue
        }
        _ => UiAction::Continue,
    }
}

fn handle_list_search_key(state: &mut UiState, code: KeyCode) -> UiAction {
    let UiScreen::List(list) = &mut state.screen else {
        return UiAction::Continue;
    };
    let changed = handle_search_key(&mut list.search, code);
    if changed {
        adjust_provider_selection(state);
    }
    UiAction::Continue
}

fn handle_template_search_key(state: &mut UiState, code: KeyCode) -> UiAction {
    let UiScreen::AddTemplate(add_template) = &mut state.screen else {
        return UiAction::Continue;
    };
    let changed = handle_search_key(&mut add_template.search, code);
    if changed {
        adjust_template_selection(state);
    }
    UiAction::Continue
}

fn handle_search_key(search: &mut SearchState, code: KeyCode) -> bool {
    match code {
        KeyCode::Enter => {
            search.active = false;
            false
        }
        KeyCode::Esc => {
            search.query.clear();
            search.active = false;
            true
        }
        KeyCode::Backspace => {
            search.query.pop();
            true
        }
        KeyCode::Char(c) if !c.is_control() => {
            search.query.push(c);
            true
        }
        _ => false,
    }
}

fn handle_add_form_key(state: &mut UiState, code: KeyCode) -> UiAction {
    match code {
        KeyCode::Esc => {
            state.screen = list_screen();
            UiAction::Continue
        }
        KeyCode::Enter => {
            let should_submit =
                matches!(&state.screen, UiScreen::AddForm(add_form) if add_form.active_field >= 5);

            if should_submit {
                let result = match &state.screen {
                    UiScreen::AddForm(add_form) => validate_add_form(&state.providers, add_form),
                    _ => return UiAction::Continue,
                };

                match result {
                    Ok(input) => UiAction::SubmitAdd(input),
                    Err(error) => {
                        if let UiScreen::AddForm(add_form) = &mut state.screen {
                            add_form.error = Some(error);
                        }
                        UiAction::Continue
                    }
                }
            } else {
                if let UiScreen::AddForm(add_form) = &mut state.screen {
                    add_form.active_field = (add_form.active_field + 1).min(5);
                }
                UiAction::Continue
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            if let UiScreen::AddForm(add_form) = &mut state.screen {
                add_form.active_field = (add_form.active_field + 1).min(5);
            }
            UiAction::Continue
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let UiScreen::AddForm(add_form) = &mut state.screen {
                add_form.active_field = add_form.active_field.saturating_sub(1);
            }
            UiAction::Continue
        }
        KeyCode::Backspace => {
            if let UiScreen::AddForm(add_form) = &mut state.screen {
                add_form.error = None;
                current_add_field(add_form).pop();
            }
            UiAction::Continue
        }
        KeyCode::Char(c) if !c.is_control() => {
            if let UiScreen::AddForm(add_form) = &mut state.screen {
                add_form.error = None;
                current_add_field(add_form).push(c);
            }
            UiAction::Continue
        }
        _ => UiAction::Continue,
    }
}

fn current_add_field(add_form: &mut AddFormState) -> &mut String {
    match add_form.active_field {
        0 => &mut add_form.name,
        1 => &mut add_form.base_url,
        2 => &mut add_form.api_key,
        3 => &mut add_form.model,
        4 => &mut add_form.notes,
        _ => &mut add_form.notes,
    }
}

fn list_search_active(state: &UiState) -> bool {
    matches!(&state.screen, UiScreen::List(list) if list.search.active)
}

fn template_search_active(state: &UiState) -> bool {
    matches!(&state.screen, UiScreen::AddTemplate(template) if template.search.active)
}

fn filtered_provider_indices(state: &UiState) -> Vec<usize> {
    let query = match &state.screen {
        UiScreen::List(list) => list.search.query.as_str(),
        _ => "",
    };

    state.providers.iter().enumerate().filter_map(|(index, provider)| {
        provider_matches_search(provider, query).then_some(index)
    }).collect()
}

fn adjust_provider_selection(state: &mut UiState) {
    let filtered = filtered_provider_indices(state);
    if filtered.is_empty() {
        state.selected = 0;
    } else if !filtered.contains(&state.selected) {
        state.selected = filtered[0];
    }
}

fn selected_provider_name(state: &UiState) -> Option<String> {
    let filtered = filtered_provider_indices(state);
    if filtered.is_empty() {
        None
    } else {
        let selected = if filtered.contains(&state.selected) {
            state.selected
        } else {
            filtered[0]
        };
        Some(state.providers[selected].name.clone())
    }
}

fn move_provider_selection(state: &mut UiState, delta: isize) {
    let filtered = filtered_provider_indices(state);
    if filtered.is_empty() {
        state.selected = 0;
        return;
    }

    let current_pos = filtered
        .iter()
        .position(|index| *index == state.selected)
        .unwrap_or(0);
    let next_pos = if delta > 0 {
        (current_pos + 1).min(filtered.len().saturating_sub(1))
    } else {
        current_pos.saturating_sub(1)
    };
    state.selected = filtered[next_pos];
}

fn filtered_template_indices(add_template: &AddTemplateState) -> Vec<usize> {
    add_template
        .choices
        .iter()
        .enumerate()
        .filter_map(|(index, choice)| {
            template_matches_search(choice, &add_template.search.query).then_some(index)
        })
        .collect()
}

fn adjust_template_selection(state: &mut UiState) {
    let UiScreen::AddTemplate(add_template) = &mut state.screen else {
        return;
    };
    let filtered = filtered_template_indices(add_template);
    if filtered.is_empty() {
        add_template.selected = 0;
    } else if !filtered.contains(&add_template.selected) {
        add_template.selected = filtered[0];
    }
}

fn move_template_selection(state: &mut UiState, delta: isize) {
    let UiScreen::AddTemplate(add_template) = &mut state.screen else {
        return;
    };
    let filtered = filtered_template_indices(add_template);
    if filtered.is_empty() {
        add_template.selected = 0;
        return;
    }

    let current_pos = filtered
        .iter()
        .position(|index| *index == add_template.selected)
        .unwrap_or(0);
    let next_pos = if delta > 0 {
        (current_pos + 1).min(filtered.len().saturating_sub(1))
    } else {
        current_pos.saturating_sub(1)
    };
    add_template.selected = filtered[next_pos];
}

fn selected_template_choice(state: &UiState) -> Option<TemplateChoice> {
    let UiScreen::AddTemplate(add_template) = &state.screen else {
        return None;
    };
    let filtered = filtered_template_indices(add_template);
    if filtered.is_empty() {
        None
    } else {
        let selected = if filtered.contains(&add_template.selected) {
            add_template.selected
        } else {
            filtered[0]
        };
        Some(add_template.choices[selected].clone())
    }
}

fn validate_add_form(
    providers: &[crate::config::ProviderConfig],
    add_form: &AddFormState,
) -> Result<crate::provider::NewProviderInput, String> {
    if add_form.name.trim().is_empty() {
        return Err("name cannot be empty".to_string());
    }
    if add_form.base_url.trim().is_empty() {
        return Err("base_url cannot be empty".to_string());
    }
    if add_form.api_key.trim().is_empty() {
        return Err("api_key cannot be empty".to_string());
    }
    if providers
        .iter()
        .any(|provider| provider.name == add_form.name)
    {
        return Err(format!("provider '{}' already exists", add_form.name));
    }

    Ok(crate::provider::NewProviderInput {
        name: add_form.name.clone(),
        base_url: add_form.base_url.clone(),
        api_key: add_form.api_key.clone(),
        model: normalize_optional(&add_form.model),
        notes: normalize_optional(&add_form.notes),
    })
}

fn normalize_optional(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn default_template_choices() -> Vec<TemplateChoice> {
    template_choices().unwrap_or_else(|_| vec![TemplateChoice::Custom])
}

fn list_screen() -> UiScreen {
    UiScreen::List(ListScreenState::default())
}

fn handle_edit_form_key(state: &mut UiState, code: KeyCode) -> UiAction {
    match code {
        KeyCode::Esc => {
            state.screen = list_screen();
            UiAction::Continue
        }
        KeyCode::Enter => {
            let should_submit = matches!(&state.screen, UiScreen::EditForm(edit_form) if edit_form.active_field >= 5);

            if should_submit {
                let result = match &state.screen {
                    UiScreen::EditForm(edit_form) => {
                        validate_edit_form(&state.providers, edit_form)
                    }
                    _ => return UiAction::Continue,
                };

                match result {
                    Ok((original_name, input)) => UiAction::SubmitEdit {
                        original_name,
                        input,
                    },
                    Err(error) => {
                        if let UiScreen::EditForm(edit_form) = &mut state.screen {
                            edit_form.error = Some(error);
                        }
                        UiAction::Continue
                    }
                }
            } else {
                if let UiScreen::EditForm(edit_form) = &mut state.screen {
                    edit_form.active_field = (edit_form.active_field + 1).min(5);
                }
                UiAction::Continue
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            if let UiScreen::EditForm(edit_form) = &mut state.screen {
                edit_form.active_field = (edit_form.active_field + 1).min(5);
            }
            UiAction::Continue
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let UiScreen::EditForm(edit_form) = &mut state.screen {
                edit_form.active_field = edit_form.active_field.saturating_sub(1);
            }
            UiAction::Continue
        }
        KeyCode::Backspace => {
            if let UiScreen::EditForm(edit_form) = &mut state.screen {
                edit_form.error = None;
                current_edit_field(edit_form).pop();
            }
            UiAction::Continue
        }
        KeyCode::Char(c) if !c.is_control() => {
            if let UiScreen::EditForm(edit_form) = &mut state.screen {
                edit_form.error = None;
                current_edit_field(edit_form).push(c);
            }
            UiAction::Continue
        }
        _ => UiAction::Continue,
    }
}

fn current_edit_field(edit_form: &mut crate::ui::state::EditFormState) -> &mut String {
    match edit_form.active_field {
        0 => &mut edit_form.name,
        1 => &mut edit_form.base_url,
        2 => &mut edit_form.api_key,
        3 => &mut edit_form.model,
        4 => &mut edit_form.notes,
        _ => &mut edit_form.notes,
    }
}

fn validate_edit_form(
    providers: &[crate::config::ProviderConfig],
    edit_form: &crate::ui::state::EditFormState,
) -> Result<(String, crate::provider::EditProviderInput), String> {
    if edit_form.name.trim().is_empty() {
        return Err("name cannot be empty".to_string());
    }
    if edit_form.base_url.trim().is_empty() {
        return Err("base_url cannot be empty".to_string());
    }
    if providers
        .iter()
        .any(|provider| provider.name == edit_form.name && provider.name != edit_form.original_name)
    {
        return Err(format!("provider '{}' already exists", edit_form.name));
    }

    Ok((
        edit_form.original_name.clone(),
        crate::provider::EditProviderInput {
            name: Some(edit_form.name.clone()),
            base_url: Some(edit_form.base_url.clone()),
            api_key: Some(edit_form.api_key.clone()),
            model: normalize_optional(&edit_form.model),
            notes: normalize_optional(&edit_form.notes),
        },
    ))
}

fn handle_delete_confirm_key(state: &mut UiState, code: KeyCode) -> UiAction {
    match code {
        KeyCode::Char('y') if !state.providers.is_empty() => {
            UiAction::DeleteSelected(state.providers[state.selected].name.clone())
        }
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('n') => {
            state.screen = list_screen();
            UiAction::Continue
        }
        _ => UiAction::Continue,
    }
}
