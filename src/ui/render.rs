use crate::ui::state::{
    AddTemplateState, SearchState, UiMode, UiScreen, UiState, provider_matches_search,
    template_matches_search,
};
use crate::ui::theme::ui_palette;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, StatefulWidget, Table, TableState};
use std::io::{IsTerminal, stdout};

pub fn provider_viewport_height(row_count: usize) -> u16 {
    (row_count as u16).saturating_add(6).clamp(10, 16)
}

pub fn terminal_width() -> Option<u16> {
    if stdout().is_terminal() {
        crossterm::terminal::size().ok().map(|(width, _)| width)
    } else {
        None
    }
}

pub fn render_provider_table(
    providers: &[crate::config::ProviderConfig],
    selected: Option<usize>,
    width: u16,
    height: u16,
) -> String {
    let mut state = UiState::new(UiMode::Manage, providers.to_vec());
    state.selected = selected.unwrap_or(0);
    render_screen(&state, width, height)
}

pub fn render_screen(state: &UiState, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width.max(60), height.max(4));
    let mut buffer = Buffer::empty(area);
    let areas = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    match &state.screen {
        UiScreen::List(_) => render_list_screen(&mut buffer, areas[0], state),
        UiScreen::DeleteConfirm => render_delete_confirm_screen(&mut buffer, areas[0], state),
        UiScreen::AddTemplate(add_template) => {
            render_add_template_screen(&mut buffer, areas[0], add_template)
        }
        UiScreen::AddForm(add_form) => render_add_form_screen(&mut buffer, areas[0], add_form),
        UiScreen::EditForm(edit_form) => render_edit_form_screen(&mut buffer, areas[0], edit_form),
    }

    ratatui::widgets::Widget::render(Paragraph::new(footer_line(state)), areas[1], &mut buffer);

    buffer_to_string(&buffer)
}

pub fn draw_screen(frame: &mut Frame<'_>, state: &UiState) {
    let areas = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());

    match &state.screen {
        UiScreen::List(_) => render_list_screen_frame(frame, areas[0], state),
        UiScreen::DeleteConfirm => render_delete_confirm_screen_frame(frame, areas[0], state),
        UiScreen::AddTemplate(add_template) => {
            render_add_template_screen_frame(frame, areas[0], add_template)
        }
        UiScreen::AddForm(add_form) => render_add_form_screen_frame(frame, areas[0], add_form),
        UiScreen::EditForm(edit_form) => render_edit_form_screen_frame(frame, areas[0], edit_form),
    }

    frame.render_widget(Paragraph::new(footer_line(state)), areas[1]);

    if let Some(position) = screen_cursor_position(state, areas[0]) {
        frame.set_cursor_position(position);
    }
}

pub fn screen_cursor_position(state: &UiState, area: Rect) -> Option<Position> {
    match &state.screen {
        UiScreen::List(list) if list.search.active => {
            Some(Position::new(area.x + 8 + list.search.query.chars().count() as u16, area.y))
        }
        UiScreen::AddTemplate(add_template) if add_template.search.active => Some(Position::new(
            area.x + 8 + add_template.search.query.chars().count() as u16,
            area.y,
        )),
        UiScreen::AddForm(add_form) => form_cursor_position(area, add_form.active_field, add_form),
        UiScreen::EditForm(edit_form) => {
            form_cursor_position(area, edit_form.active_field, edit_form)
        }
        _ => None,
    }
}

fn render_list_screen(buffer: &mut Buffer, area: Rect, state: &UiState) {
    let filtered = filtered_providers(state);
    let search = match &state.screen {
        UiScreen::List(list) => Some(&list.search),
        _ => None,
    };
    let show_search = search.is_some_and(show_search_bar);

    let content_areas = if show_search {
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(0)]).split(area)
    };

    if let Some(search) = search.filter(|search| show_search_bar(search)) {
        render_search_bar(buffer, content_areas[0], search);
    }

    let list_area = if show_search {
        content_areas[1]
    } else {
        content_areas[0]
    };

    if filtered.is_empty() {
        let empty_copy = if search.is_some_and(|search| !search.query.is_empty()) {
            "No providers match the current search"
        } else {
            "No providers configured"
        };
        ratatui::widgets::Widget::render(
            Paragraph::new(empty_copy.to_string())
                .style(Style::default().fg(ui_palette().body_fg))
                .block(table_block("Providers")),
            list_area,
            buffer,
        );
        return;
    }

    let table = build_provider_table(&filtered, list_area.width, selected_filtered_provider_index(state, &filtered), "Providers");
    let mut table_state = TableState::default();
    table_state.select(selected_filtered_provider_index(state, &filtered));
    StatefulWidget::render(table, list_area, buffer, &mut table_state);
}

fn render_delete_confirm_screen(buffer: &mut Buffer, area: Rect, state: &UiState) {
    let provider_name = state
        .providers
        .get(state.selected)
        .map(|provider| provider.name.as_str())
        .unwrap_or("provider");

    render_placeholder_screen(
        buffer,
        area,
        "Delete Provider",
        &format!("Delete provider {provider_name}?"),
    );
}

fn render_add_template_screen(
    buffer: &mut Buffer,
    area: Rect,
    add_template: &AddTemplateState,
) {
    let filtered = filtered_templates(add_template);
    let show_search = show_search_bar(&add_template.search);
    let content_areas = if show_search {
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(0)]).split(area)
    };

    if show_search {
        render_search_bar(buffer, content_areas[0], &add_template.search);
    }

    let list_area = if show_search {
        content_areas[1]
    } else {
        content_areas[0]
    };

    if filtered.is_empty() {
        ratatui::widgets::Widget::render(
            Paragraph::new("No templates match the current search".to_string())
                .style(Style::default().fg(ui_palette().body_fg))
                .block(table_block("Provider Templates")),
            list_area,
            buffer,
        );
        return;
    }

    let table = build_template_table(
        &filtered,
        list_area.width,
        selected_filtered_template_index(add_template, &filtered),
        "Provider Templates",
    );
    let mut table_state = TableState::default();
    table_state.select(selected_filtered_template_index(add_template, &filtered));
    StatefulWidget::render(table, list_area, buffer, &mut table_state);
}

fn render_add_form_screen(
    buffer: &mut Buffer,
    area: Rect,
    add_form: &crate::ui::state::AddFormState,
) {
    ratatui::widgets::Widget::render(build_add_form_paragraph(add_form), area, buffer);
}

fn render_edit_form_screen(
    buffer: &mut Buffer,
    area: Rect,
    edit_form: &crate::ui::state::EditFormState,
) {
    ratatui::widgets::Widget::render(build_edit_form_paragraph(edit_form), area, buffer);
}

fn render_placeholder_screen(buffer: &mut Buffer, area: Rect, title: &'static str, body: &str) {
    ratatui::widgets::Widget::render(
        Paragraph::new(body.to_string())
            .style(Style::default().fg(ui_palette().body_fg))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ui_palette().border))
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(ui_palette().title)
                            .add_modifier(Modifier::BOLD),
                    ),
            ),
        area,
        buffer,
    );
}

fn render_list_screen_frame(frame: &mut Frame<'_>, area: Rect, state: &UiState) {
    let filtered = filtered_providers(state);
    let search = match &state.screen {
        UiScreen::List(list) => Some(&list.search),
        _ => None,
    };
    let show_search = search.is_some_and(show_search_bar);

    let content_areas = if show_search {
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(0)]).split(area)
    };

    if let Some(search) = search.filter(|search| show_search_bar(search)) {
        render_search_bar_frame(frame, content_areas[0], search);
    }

    let list_area = if show_search {
        content_areas[1]
    } else {
        content_areas[0]
    };

    if filtered.is_empty() {
        let empty_copy = if search.is_some_and(|search| !search.query.is_empty()) {
            "No providers match the current search"
        } else {
            "No providers configured"
        };
        frame.render_widget(
            Paragraph::new(empty_copy.to_string())
                .style(Style::default().fg(ui_palette().body_fg))
                .block(table_block("Providers")),
            list_area,
        );
        return;
    }

    let table = build_provider_table(&filtered, list_area.width, selected_filtered_provider_index(state, &filtered), "Providers");
    let mut table_state = TableState::default();
    table_state.select(selected_filtered_provider_index(state, &filtered));
    frame.render_stateful_widget(table, list_area, &mut table_state);
}

fn render_delete_confirm_screen_frame(frame: &mut Frame<'_>, area: Rect, state: &UiState) {
    let provider_name = state
        .providers
        .get(state.selected)
        .map(|provider| provider.name.as_str())
        .unwrap_or("provider");

    render_placeholder_screen_frame(
        frame,
        area,
        "Delete Provider",
        &format!("Delete provider {provider_name}?"),
    );
}

fn render_add_template_screen_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    add_template: &AddTemplateState,
) {
    let filtered = filtered_templates(add_template);
    let show_search = show_search_bar(&add_template.search);
    let content_areas = if show_search {
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(0)]).split(area)
    };

    if show_search {
        render_search_bar_frame(frame, content_areas[0], &add_template.search);
    }

    let list_area = if show_search {
        content_areas[1]
    } else {
        content_areas[0]
    };

    if filtered.is_empty() {
        frame.render_widget(
            Paragraph::new("No templates match the current search".to_string())
                .style(Style::default().fg(ui_palette().body_fg))
                .block(table_block("Provider Templates")),
            list_area,
        );
        return;
    }

    let table = build_template_table(
        &filtered,
        list_area.width,
        selected_filtered_template_index(add_template, &filtered),
        "Provider Templates",
    );
    let mut table_state = TableState::default();
    table_state.select(selected_filtered_template_index(add_template, &filtered));
    frame.render_stateful_widget(table, list_area, &mut table_state);
}

fn render_add_form_screen_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    add_form: &crate::ui::state::AddFormState,
) {
    frame.render_widget(build_add_form_paragraph(add_form), area);
}

fn render_edit_form_screen_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    edit_form: &crate::ui::state::EditFormState,
) {
    frame.render_widget(build_edit_form_paragraph(edit_form), area);
}

fn render_placeholder_screen_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    body: &str,
) {
    frame.render_widget(
        Paragraph::new(body.to_string())
            .style(Style::default().fg(ui_palette().body_fg))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ui_palette().border))
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(ui_palette().title)
                            .add_modifier(Modifier::BOLD),
                    ),
            ),
        area,
    );
}

struct HintToken<'a> {
    key: &'a str,
    description: &'a str,
}

fn footer_line(state: &UiState) -> Line<'static> {
    match &state.screen {
        UiScreen::DeleteConfirm => hint_line(&[
            HintToken {
                key: "Y",
                description: "confirm",
            },
            HintToken {
                key: "Esc",
                description: "cancel",
            },
        ]),
        UiScreen::List(list) if list.search.active => hint_line(&[
            HintToken {
                key: "Type",
                description: "filter",
            },
            HintToken {
                key: "Enter",
                description: "close",
            },
            HintToken {
                key: "Esc",
                description: "clear",
            },
        ]),
        UiScreen::List(_) => match state.mode {
            UiMode::Exec => hint_line(&[
                HintToken {
                    key: "Enter",
                    description: "exec",
                },
                HintToken {
                    key: "/",
                    description: "search",
                },
                HintToken {
                    key: "N",
                    description: "new",
                },
                HintToken {
                    key: "E",
                    description: "edit",
                },
                HintToken {
                    key: "D",
                    description: "delete",
                },
                HintToken {
                    key: "Q",
                    description: "quit",
                },
            ]),
            UiMode::Manage | UiMode::Run => hint_line(&[
                HintToken {
                    key: "Enter",
                    description: "run",
                },
                HintToken {
                    key: "/",
                    description: "search",
                },
                HintToken {
                    key: "N",
                    description: "new",
                },
                HintToken {
                    key: "E",
                    description: "edit",
                },
                HintToken {
                    key: "D",
                    description: "delete",
                },
                HintToken {
                    key: "Q",
                    description: "quit",
                },
            ]),
        },
        UiScreen::AddTemplate(template) if template.search.active => hint_line(&[
            HintToken {
                key: "Type",
                description: "filter",
            },
            HintToken {
                key: "Enter",
                description: "close",
            },
            HintToken {
                key: "Esc",
                description: "clear",
            },
        ]),
        UiScreen::AddTemplate(_) => hint_line(&[
            HintToken {
                key: "Enter",
                description: "select",
            },
            HintToken {
                key: "/",
                description: "search",
            },
            HintToken {
                key: "Esc",
                description: "cancel",
            },
        ]),
        UiScreen::AddForm(_) | UiScreen::EditForm(_) => hint_line(&[
            HintToken {
                key: "Tab",
                description: "next",
            },
            HintToken {
                key: "Shift+Tab",
                description: "back",
            },
            HintToken {
                key: "Enter",
                description: "save",
            },
            HintToken {
                key: "Esc",
                description: "cancel",
            },
        ]),
    }
}

fn render_search_bar(buffer: &mut Buffer, area: Rect, search: &SearchState) {
    ratatui::widgets::Widget::render(search_bar(search), area, buffer);
}

fn render_search_bar_frame(frame: &mut Frame<'_>, area: Rect, search: &SearchState) {
    frame.render_widget(search_bar(search), area);
}

fn search_bar(search: &SearchState) -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled("Search: ", Style::default().fg(ui_palette().label_fg)),
        Span::styled(search.query.clone(), Style::default().fg(ui_palette().value_fg)),
    ]))
}

fn hint_line(tokens: &[HintToken<'_>]) -> Line<'static> {
    let mut spans = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!("[{}]", token.key),
            Style::default()
                .fg(ui_palette().keycap_fg)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            token.description.to_string(),
            Style::default().fg(ui_palette().help_fg),
        ));
    }

    Line::from(spans)
}

fn table_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui_palette().border))
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(ui_palette().table_title_fg)
                .add_modifier(Modifier::BOLD),
        )
}

fn selected_row_style() -> Style {
    Style::default()
        .fg(ui_palette().selected_fg)
        .bg(ui_palette().selected_bg)
        .add_modifier(Modifier::BOLD)
}

fn build_provider_table(
    providers: &[&crate::config::ProviderConfig],
    width: u16,
    selected: Option<usize>,
    title: &'static str,
) -> Table<'static> {
    let header = Row::new(vec![Cell::from(""), Cell::from("Name"), Cell::from("Base URL")]).style(
        Style::default()
            .fg(ui_palette().header_fg)
            .add_modifier(Modifier::BOLD),
    );

    let rows = providers.iter().enumerate().map(|(index, provider)| {
        let marker = if selected == Some(index) { ">" } else { " " };

        Row::new(vec![
            Cell::from(marker),
            Cell::from(provider.name.clone()),
            Cell::from(provider.base_url.clone()),
        ])
            .style(Style::default().fg(ui_palette().table_item_fg))
    });

    let widths = if width >= 100 {
        vec![
            Constraint::Length(2),
            Constraint::Length(22),
            Constraint::Min(24),
        ]
    } else {
        vec![
            Constraint::Length(2),
            Constraint::Length(16),
            Constraint::Min(18),
        ]
    };

    Table::new(rows, widths)
        .header(header)
        .block(table_block(title))
        .row_highlight_style(selected_row_style())
}

fn build_template_table(
    choices: &[&crate::provider_templates::TemplateChoice],
    width: u16,
    selected: Option<usize>,
    title: &'static str,
) -> Table<'static> {
    let header = Row::new(vec![Cell::from(""), Cell::from("Name"), Cell::from("Base URL")]).style(
        Style::default()
            .fg(ui_palette().header_fg)
            .add_modifier(Modifier::BOLD),
    );

    let rows = choices.iter().enumerate().map(|(index, choice)| match choice {
        crate::provider_templates::TemplateChoice::BuiltIn(template) => {
            let marker = if selected == Some(index) { ">" } else { " " };

            Row::new(vec![
                Cell::from(marker),
                Cell::from(template.display_name.clone()),
                Cell::from(template.base_url.clone()),
            ])
                .style(Style::default().fg(ui_palette().table_item_fg))
        }
        crate::provider_templates::TemplateChoice::Custom => {
            let marker = if selected == Some(index) { ">" } else { " " };

            Row::new(vec![
                Cell::from(marker),
                Cell::from("Custom").style(Style::default().fg(ui_palette().custom_accent)),
                Cell::from("Manual endpoint setup"),
            ])
            .style(Style::default().fg(ui_palette().table_item_fg))
        }
    });

    let widths = if width >= 100 {
        vec![
            Constraint::Length(2),
            Constraint::Length(22),
            Constraint::Min(24),
        ]
    } else {
        vec![
            Constraint::Length(2),
            Constraint::Length(16),
            Constraint::Min(18),
        ]
    };

    Table::new(rows, widths)
        .header(header)
        .block(table_block(title))
        .row_highlight_style(selected_row_style())
}

fn show_search_bar(search: &SearchState) -> bool {
    search.active || !search.query.is_empty()
}

fn filtered_providers(state: &UiState) -> Vec<&crate::config::ProviderConfig> {
    let query = match &state.screen {
        UiScreen::List(list) => list.search.query.as_str(),
        _ => "",
    };

    state.providers.iter().filter(|provider| provider_matches_search(provider, query)).collect()
}

fn selected_filtered_provider_index(
    state: &UiState,
    filtered: &[&crate::config::ProviderConfig],
) -> Option<usize> {
    if filtered.is_empty() {
        return None;
    }

    filtered
        .iter()
        .position(|provider| provider.name == state.providers[state.selected].name)
        .or(Some(0))
}

fn filtered_templates<'a>(
    add_template: &'a AddTemplateState,
) -> Vec<&'a crate::provider_templates::TemplateChoice> {
    add_template
        .choices
        .iter()
        .filter(|choice| template_matches_search(choice, &add_template.search.query))
        .collect()
}

fn selected_filtered_template_index(
    add_template: &AddTemplateState,
    filtered: &[&crate::provider_templates::TemplateChoice],
) -> Option<usize> {
    if filtered.is_empty() {
        return None;
    }

    filtered
        .iter()
        .position(|choice| {
            add_template
                .choices
                .get(add_template.selected)
                .map(|selected| selected == *choice)
                .unwrap_or(false)
        })
        .or(Some(0))
}

fn build_add_form_paragraph(add_form: &crate::ui::state::AddFormState) -> Paragraph<'static> {
    build_form_paragraph(
        "Add Provider",
        vec![
            form_field_line(
                0,
                add_form.active_field,
                "Name",
                &text_field_display(&add_form.name, true),
            ),
            form_field_line(
                1,
                add_form.active_field,
                "Base URL",
                &text_field_display(&add_form.base_url, true),
            ),
            form_field_line(
                2,
                add_form.active_field,
                "API Key",
                &secret_field_display(&add_form.api_key, "required", add_form.active_field == 2),
            ),
            form_field_line(
                3,
                add_form.active_field,
                "Model",
                &text_field_display(&add_form.model, add_form.active_field == 3),
            ),
            form_field_line(
                4,
                add_form.active_field,
                "Notes",
                &text_field_display(&add_form.notes, add_form.active_field == 4),
            ),
            save_line(add_form.active_field == 5),
            error_line(add_form.error.as_deref()),
        ],
    )
}

fn build_edit_form_paragraph(edit_form: &crate::ui::state::EditFormState) -> Paragraph<'static> {
    build_form_paragraph(
        "Edit Provider",
        vec![
            form_field_line(
                0,
                edit_form.active_field,
                "Name",
                &text_field_display(&edit_form.name, true),
            ),
            form_field_line(
                1,
                edit_form.active_field,
                "Base URL",
                &text_field_display(&edit_form.base_url, true),
            ),
            form_field_line(
                2,
                edit_form.active_field,
                "API Key",
                &secret_field_display(
                    &edit_form.api_key,
                    "(keep current)",
                    edit_form.active_field == 2,
                ),
            ),
            form_field_line(
                3,
                edit_form.active_field,
                "Model",
                &text_field_display(&edit_form.model, edit_form.active_field == 3),
            ),
            form_field_line(
                4,
                edit_form.active_field,
                "Notes",
                &text_field_display(&edit_form.notes, edit_form.active_field == 4),
            ),
            save_line(edit_form.active_field == 5),
            error_line(edit_form.error.as_deref()),
        ],
    )
}

fn build_form_paragraph(title: &'static str, lines: Vec<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ui_palette().border))
            .title(title)
            .title_style(
                Style::default()
                    .fg(ui_palette().title)
                    .add_modifier(Modifier::BOLD),
            ),
    )
}

fn form_field_line(index: usize, active_field: usize, label: &str, value: &str) -> Line<'static> {
    let active = index == active_field;
    let prefix = if active { "› " } else { "  " };
    let prefix_style = if active {
        Style::default()
            .fg(ui_palette().custom_accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui_palette().help_fg)
    };
    let label_style = if active {
        Style::default()
            .fg(ui_palette().label_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui_palette().label_fg)
    };
    let value_style = if active {
        Style::default()
            .fg(ui_palette().value_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui_palette().value_fg)
    };

    Line::from(vec![
        Span::styled(prefix.to_string(), prefix_style),
        Span::styled(format!("{label:8}"), label_style),
        Span::raw(" "),
        Span::styled(value.to_string(), value_style),
    ])
}

fn save_line(active: bool) -> Line<'static> {
    let prefix = if active { "› " } else { "  " };
    let prefix_style = if active {
        Style::default()
            .fg(ui_palette().custom_accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui_palette().help_fg)
    };
    let text_style = if active {
        Style::default()
            .fg(ui_palette().value_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui_palette().body_fg)
    };

    Line::from(vec![
        Span::styled(prefix.to_string(), prefix_style),
        Span::styled("[ Save Provider ]".to_string(), text_style),
    ])
}

fn error_line(error: Option<&str>) -> Line<'static> {
    match error {
        Some(message) => Line::from(Span::styled(
            message.to_string(),
            Style::default()
                .fg(ui_palette().error_fg)
                .add_modifier(Modifier::BOLD),
        )),
        None => Line::from(""),
    }
}

fn text_field_display(value: &str, active: bool) -> String {
    if value.is_empty() && !active {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn secret_field_display(value: &str, placeholder: &str, active: bool) -> String {
    if value.is_empty() {
        if active {
            String::new()
        } else {
            placeholder.to_string()
        }
    } else {
        "*".repeat(value.chars().count())
    }
}

fn form_cursor_position<F>(area: Rect, active_field: usize, form: &F) -> Option<Position>
where
    F: FormCursorValue,
{
    if active_field > 4 {
        return None;
    }

    let field_value_len = form.cursor_value_len(active_field) as u16;
    Some(Position::new(area.x + 12 + field_value_len, area.y + 1 + active_field as u16))
}

trait FormCursorValue {
    fn cursor_value_len(&self, active_field: usize) -> usize;
}

impl FormCursorValue for crate::ui::state::AddFormState {
    fn cursor_value_len(&self, active_field: usize) -> usize {
        match active_field {
            0 => self.name.chars().count(),
            1 => self.base_url.chars().count(),
            2 => self.api_key.chars().count(),
            3 => self.model.chars().count(),
            4 => self.notes.chars().count(),
            _ => 0,
        }
    }
}

impl FormCursorValue for crate::ui::state::EditFormState {
    fn cursor_value_len(&self, active_field: usize) -> usize {
        match active_field {
            0 => self.name.chars().count(),
            1 => self.base_url.chars().count(),
            2 => self.api_key.chars().count(),
            3 => self.model.chars().count(),
            4 => self.notes.chars().count(),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HintToken, build_template_table, form_field_line, hint_line, render_list_screen,
    };
    use crate::config::ProviderConfig;
    use crate::provider_templates::{ProviderTemplate, TemplateChoice};
    use crate::ui::state::{UiMode, UiState};
    use crate::ui::theme::ui_palette;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;
    use ratatui::widgets::{StatefulWidget, TableState};

    fn provider_named(name: &str, base_url: &str) -> ProviderConfig {
        ProviderConfig {
            name: name.into(),
            base_url: base_url.into(),
            env_key: "CXCM_KEY_12345678".into(),
            api_key: "secret".into(),
            model: Some("gpt-5.4".into()),
            last_used_at: None,
            notes: Some("internal relay".into()),
        }
    }

    fn color_for_symbol_in_row(buffer: &Buffer, row: u16, symbol: &str) -> Option<Color> {
        (0..buffer.area.width).find_map(|column| {
            buffer.cell((column, row)).and_then(|cell| {
                if cell.symbol() == symbol {
                    Some(cell.fg)
                } else {
                    None
                }
            })
        })
    }

    fn color_for_symbol_after_row(buffer: &Buffer, row_start: u16, symbol: &str) -> Option<Color> {
        (row_start..buffer.area.height)
            .find_map(|row| color_for_symbol_in_row(buffer, row, symbol))
    }

    #[test]
    fn form_field_line_uses_distinct_label_and_value_styles() {
        let line = form_field_line(0, 0, "Name", "openai");

        assert!(line.spans.len() >= 4);
        assert_eq!(line.spans[1].style.fg, Some(ui_palette().label_fg));
        assert_eq!(line.spans[3].style.fg, Some(ui_palette().value_fg));
    }

    #[test]
    fn hint_line_renders_keycaps_without_background_fill() {
        let line = hint_line(&[HintToken {
            key: "Enter",
            description: "run",
        }]);

        assert_eq!(line.spans[0].style.bg, None);
    }

    #[test]
    fn provider_table_title_and_items_use_distinct_palette_levels() {
        let area = Rect::new(0, 0, 72, 6);
        let mut buffer = Buffer::empty(area);
        let state = UiState::new(
            UiMode::Run,
            vec![
                provider_named("router-a", "https://router.example.com/v1"),
                provider_named("backup", "https://backup.example.com/v1"),
            ],
        );

        render_list_screen(&mut buffer, area, &state);

        let title_color = color_for_symbol_in_row(&buffer, 0, "P").unwrap();
        let item_color = color_for_symbol_after_row(&buffer, 1, "b").unwrap();

        assert_eq!(title_color, ui_palette().table_title_fg);
        assert_eq!(item_color, ui_palette().table_item_fg);
        assert_ne!(title_color, item_color);
    }

    #[test]
    fn template_table_title_and_items_use_distinct_palette_levels() {
        let area = Rect::new(0, 0, 72, 6);
        let mut buffer = Buffer::empty(area);
        let choices = vec![TemplateChoice::BuiltIn(ProviderTemplate {
            id: "ggboom".into(),
            display_name: "ggboom".into(),
            base_url: "https://ai.qaq.al".into(),
            default_model: Some("gpt-5.4".into()),
            notes: Some("relay".into()),
        }), TemplateChoice::Custom];
        let table = build_template_table(
            &choices.iter().collect::<Vec<_>>(),
            area.width,
            Some(1),
            "Provider Templates",
        );
        let mut state = TableState::default();
        state.select(Some(1));

        StatefulWidget::render(table, area, &mut buffer, &mut state);

        let title_color = color_for_symbol_in_row(&buffer, 0, "P").unwrap();
        let item_color = color_for_symbol_after_row(&buffer, 1, "g").unwrap();

        assert_eq!(title_color, ui_palette().table_title_fg);
        assert_eq!(item_color, ui_palette().table_item_fg);
        assert_ne!(title_color, item_color);
    }
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::new();

    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        lines.push(line.trim_end().to_string());
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}
