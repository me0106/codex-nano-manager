use crossterm::event::KeyCode;
use codex_nano_manager::config::ProviderConfig;
use codex_nano_manager::ui::action::UiAction;
use codex_nano_manager::ui::input::handle_key;
use codex_nano_manager::ui::render::{
    provider_viewport_height, render_screen, screen_cursor_position,
};
use codex_nano_manager::ui::state::{AddFormState, EditFormState, UiMode, UiScreen, UiState};
use codex_nano_manager::ui::theme::ui_palette;
use ratatui::layout::Rect;
use ratatui::style::Color;

fn provider() -> ProviderConfig {
    ProviderConfig {
        name: "router-a".into(),
        base_url: "https://router.example.com/v1".into(),
        env_key: "CXCM_KEY_12345678".into(),
        api_key: "secret".into(),
        model: Some("gpt-5.4".into()),
        last_used_at: None,
        notes: Some("internal relay".into()),
    }
}

#[test]
fn ui_mode_supports_manage_run_and_exec() {
    let modes = [UiMode::Manage, UiMode::Run, UiMode::Exec];
    assert_eq!(modes.len(), 3);
}

#[test]
fn keycap_palette_colors_are_defined_and_distinct() {
    let palette = ui_palette();

    assert_ne!(palette.keycap_fg, palette.help_fg);
    assert_eq!(palette.keycap_bg, Color::Reset);
}

#[test]
fn ui_screen_starts_in_list_mode() {
    let state = UiState::new(UiMode::Run, Vec::new());
    let UiScreen::List(list) = state.screen else {
        panic!("expected list screen");
    };
    assert_eq!(list.search.query, "");
    assert!(!list.search.active);
}

#[test]
fn list_screen_enter_runs_in_manage_and_run_modes() {
    let providers = vec![provider()];
    let mut state = UiState::new(UiMode::Run, providers);

    let action = handle_key(&mut state, KeyCode::Enter);

    assert!(matches!(action, UiAction::RunSelected(name) if name == "router-a"));
}

#[test]
fn list_screen_enter_execs_in_exec_mode() {
    let providers = vec![provider()];
    let mut state = UiState::new(UiMode::Exec, providers);

    let action = handle_key(&mut state, KeyCode::Enter);

    assert!(matches!(action, UiAction::ExecSelected(name) if name == "router-a"));
}

#[test]
fn list_screen_n_switches_to_add_template_screen() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);

    let action = handle_key(&mut state, KeyCode::Char('n'));

    assert!(matches!(action, UiAction::Continue));
    assert!(matches!(state.screen, UiScreen::AddTemplate(_)));
}

#[test]
fn slash_enters_search_mode_on_provider_list() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);

    let action = handle_key(&mut state, KeyCode::Char('/'));

    let UiScreen::List(list) = &state.screen else {
        panic!("expected list screen");
    };

    assert!(matches!(action, UiAction::Continue));
    assert!(list.search.active);
}

#[test]
fn typing_updates_provider_list_search_query() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    let _ = handle_key(&mut state, KeyCode::Char('/'));

    let _ = handle_key(&mut state, KeyCode::Char('r'));
    let _ = handle_key(&mut state, KeyCode::Char('o'));

    let UiScreen::List(list) = &state.screen else {
        panic!("expected list screen");
    };

    assert_eq!(list.search.query, "ro");
}

#[test]
fn escape_clears_provider_list_search_and_exits_mode() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    let _ = handle_key(&mut state, KeyCode::Char('/'));
    let _ = handle_key(&mut state, KeyCode::Char('r'));

    let action = handle_key(&mut state, KeyCode::Esc);

    let UiScreen::List(list) = &state.screen else {
        panic!("expected list screen");
    };

    assert!(matches!(action, UiAction::Continue));
    assert_eq!(list.search.query, "");
    assert!(!list.search.active);
}

#[test]
fn slash_enters_search_mode_on_template_list() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));

    let action = handle_key(&mut state, KeyCode::Char('/'));

    let UiScreen::AddTemplate(template) = &state.screen else {
        panic!("expected template screen");
    };

    assert!(matches!(action, UiAction::Continue));
    assert!(template.search.active);
}

#[test]
fn typing_updates_template_search_query() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));
    let _ = handle_key(&mut state, KeyCode::Char('/'));
    let _ = handle_key(&mut state, KeyCode::Char('o'));

    let UiScreen::AddTemplate(template) = &state.screen else {
        panic!("expected template screen");
    };

    assert_eq!(template.search.query, "o");
}

#[test]
fn provider_search_filters_non_matching_rows() {
    let mut state = UiState::new(
        UiMode::Run,
        vec![
            provider(),
            ProviderConfig {
                name: "other".into(),
                base_url: "https://example.com".into(),
                env_key: "CXCM_KEY_22222222".into(),
                api_key: "secret-2".into(),
                model: None,
                last_used_at: None,
                notes: Some("backup".into()),
            },
        ],
    );
    let _ = handle_key(&mut state, KeyCode::Char('/'));
    for c in "router".chars() {
        let _ = handle_key(&mut state, KeyCode::Char(c));
    }
    let output = render_screen(&state, 120, 12);

    assert!(output.contains("router-a"));
    assert!(!output.contains("https://example.com"));
}

#[test]
fn template_search_filters_to_custom_row() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));
    let _ = handle_key(&mut state, KeyCode::Char('/'));
    let _ = handle_key(&mut state, KeyCode::Char('c'));
    let output = render_screen(&state, 120, 12);

    assert!(output.contains("Custom"));
}

#[test]
fn provider_search_bar_renders_when_active() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    let _ = handle_key(&mut state, KeyCode::Char('/'));
    let _ = handle_key(&mut state, KeyCode::Char('r'));
    let output = render_screen(&state, 120, 12);

    assert!(output.contains("Search:"));
    assert!(output.contains("r"));
    assert!(output.contains("[Enter]"));
    assert!(output.contains("[Esc]"));
}

#[test]
fn provider_search_renders_no_match_empty_state() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    let _ = handle_key(&mut state, KeyCode::Char('/'));
    let _ = handle_key(&mut state, KeyCode::Char('z'));
    let output = render_screen(&state, 120, 12);

    assert!(output.contains("No providers match the current search"));
}

#[test]
fn provider_search_cursor_points_to_end_of_query() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    let _ = handle_key(&mut state, KeyCode::Char('/'));
    let _ = handle_key(&mut state, KeyCode::Char('o'));
    let _ = handle_key(&mut state, KeyCode::Char('p'));

    let pos = screen_cursor_position(&state, Rect::new(0, 0, 120, 11));

    assert_eq!(pos.map(|p| (p.x, p.y)), Some((10, 0)));
}

#[test]
fn render_list_screen_shows_provider_table_and_help() {
    let state = UiState::new(UiMode::Run, vec![provider()]);
    let output = render_screen(&state, 120, 12);

    assert!(output.contains("Providers"));
    assert!(output.contains("router-a"));
    assert!(output.contains("[Enter]"));
    assert!(output.contains("run"));
}

#[test]
fn render_list_screen_aligns_base_url_column_across_rows() {
    let mut state = UiState::new(
        UiMode::Run,
        vec![
            provider(),
            ProviderConfig {
                name: "much-longer-provider".into(),
                base_url: "https://second.example.com/v1".into(),
                env_key: "CXCM_KEY_87654321".into(),
                api_key: "secret-2".into(),
                model: Some("gpt-5.4".into()),
                last_used_at: None,
                notes: Some("backup".into()),
            },
        ],
    );
    state.selected = 0;

    let output = render_screen(&state, 120, 12);
    let rows: Vec<_> = output
        .lines()
        .filter(|line| line.contains("https://router.example.com/v1") || line.contains("https://second.example.com/v1"))
        .collect();

    assert_eq!(rows.len(), 2);
    let first = rows[0].find("https://").unwrap();
    let second = rows[1].find("https://").unwrap();
    assert_eq!(first, second);
}

#[test]
fn render_delete_confirm_screen_replaces_table_content() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    state.screen = UiScreen::DeleteConfirm;

    let output = render_screen(&state, 120, 10);

    assert!(output.contains("Delete Provider"));
    assert!(!output.contains("internal relay"));
}

#[test]
fn provider_runtime_viewport_stays_compact() {
    assert!(provider_viewport_height(20) <= 16);
}

#[test]
fn add_template_selects_builtin_and_enters_add_form() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));

    let action = handle_key(&mut state, KeyCode::Enter);

    assert!(matches!(action, UiAction::Continue));
    assert!(matches!(state.screen, UiScreen::AddForm(_)));
}

#[test]
fn add_form_escape_returns_to_list() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    state.screen = UiScreen::AddForm(AddFormState::custom());

    let action = handle_key(&mut state, KeyCode::Esc);

    assert!(matches!(action, UiAction::Continue));
    assert!(matches!(state.screen, UiScreen::List(_)));
}

#[test]
fn add_form_submit_emits_submit_add_action() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    state.screen = UiScreen::AddForm(AddFormState::ready_to_submit(
        "ggboom",
        "https://ai.qaq.al",
        "sk-test",
    ));

    let action = handle_key(&mut state, KeyCode::Enter);

    assert!(matches!(action, UiAction::SubmitAdd(input) if input.name == "ggboom"));
}

#[test]
fn render_add_template_screen_shows_builtin_choices() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));

    let output = render_screen(&state, 120, 12);

    assert!(output.contains("Provider Templates"));
    assert!(output.contains("GGBoom"));
}

#[test]
fn render_add_form_screen_shows_fields_and_prefilled_values() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));
    let _ = handle_key(&mut state, KeyCode::Enter);

    let output = render_screen(&state, 120, 12);

    assert!(output.contains("Add Provider"));
    assert!(output.contains("Name"));
    assert!(output.contains("ggboom"));
}

#[test]
fn render_add_form_marks_active_field() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));
    let _ = handle_key(&mut state, KeyCode::Enter);

    let output = render_screen(&state, 120, 12);

    assert!(output.contains("› API Key"));
    assert!(output.contains("[Tab]"));
    assert!(output.contains("[Shift+Tab]"));
}

#[test]
fn add_form_cursor_position_points_to_active_input() {
    let mut state = UiState::new(UiMode::Run, vec![]);
    let _ = handle_key(&mut state, KeyCode::Char('n'));
    let _ = handle_key(&mut state, KeyCode::Enter);

    let pos = screen_cursor_position(&state, Rect::new(0, 0, 120, 11));

    assert_eq!(pos.map(|p| (p.x, p.y)), Some((12, 3)));
}

#[test]
fn render_edit_form_screen_shows_fields_without_provider_table() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    let _ = handle_key(&mut state, KeyCode::Char('e'));

    let output = render_screen(&state, 120, 12);

    assert!(output.contains("Edit Provider"));
    assert!(output.contains("Base URL"));
    assert!(!output.contains("Providers"));
}

#[test]
fn edit_key_enters_edit_form_with_selected_provider_values() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);

    let action = handle_key(&mut state, KeyCode::Char('e'));

    assert!(matches!(action, UiAction::Continue));
    assert!(matches!(state.screen, UiScreen::EditForm(_)));
}

#[test]
fn edit_form_submit_emits_submit_edit_action() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    state.screen = UiScreen::EditForm(EditFormState::ready_to_submit(&provider(), "router-b"));

    let action = handle_key(&mut state, KeyCode::Enter);

    assert!(
        matches!(action, UiAction::SubmitEdit { original_name, .. } if original_name == "router-a")
    );
}

#[test]
fn list_screen_d_enters_delete_confirm() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);

    let action = handle_key(&mut state, KeyCode::Char('d'));

    assert!(matches!(action, UiAction::Continue));
    assert!(matches!(state.screen, UiScreen::DeleteConfirm));
}

#[test]
fn delete_confirm_y_emits_delete_selected() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    state.screen = UiScreen::DeleteConfirm;

    let action = handle_key(&mut state, KeyCode::Char('y'));

    assert!(matches!(action, UiAction::DeleteSelected(name) if name == "router-a"));
}

#[test]
fn delete_confirm_escape_returns_to_list() {
    let mut state = UiState::new(UiMode::Run, vec![provider()]);
    state.screen = UiScreen::DeleteConfirm;

    let action = handle_key(&mut state, KeyCode::Esc);

    assert!(matches!(action, UiAction::Continue));
    assert!(matches!(state.screen, UiScreen::List(_)));
}
