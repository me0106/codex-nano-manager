use crate::config::ProviderConfig;
use crate::error::AppError;
use crate::provider_templates::TemplateChoice;
use crate::ui::TerminalCleanupGuard;
use crate::ui::theme::ui_palette;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::terminal::enable_raw_mode;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::{Terminal, TerminalOptions, Viewport};
use std::io::{Stdout, stdout};

pub trait ProviderSelector {
    fn select(&self, providers: &[ProviderConfig]) -> Result<String, AppError>;
}

pub struct RatatuiSelector;

impl ProviderSelector for RatatuiSelector {
    fn select(&self, providers: &[ProviderConfig]) -> Result<String, AppError> {
        enable_raw_mode()?;
        let _cleanup = TerminalCleanupGuard::new();

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: selector_viewport(providers.len()),
            },
        )?;
        run_provider_selector_loop(&mut terminal, providers)
    }
}

pub fn select_template_choice(choices: &[TemplateChoice]) -> Result<TemplateChoice, AppError> {
    enable_raw_mode()?;
    let _cleanup = TerminalCleanupGuard::new();

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: selector_viewport(choices.len()),
        },
    )?;
    run_template_selector_loop(&mut terminal, choices)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectorState {
    selected: usize,
    row_count: usize,
}

impl SelectorState {
    fn new(row_count: usize) -> Self {
        Self {
            selected: 0,
            row_count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionAction {
    Continue,
    Submit,
    Cancel,
}

fn selector_viewport(row_count: usize) -> Viewport {
    Viewport::Inline((row_count as u16).saturating_add(4).clamp(6, 12))
}

fn should_process_key_event(key: KeyEvent) -> bool {
    matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

fn handle_key_code(state: &mut SelectorState, code: KeyCode) -> SelectionAction {
    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            if state.row_count > 0 {
                state.selected = (state.selected + 1).min(state.row_count.saturating_sub(1));
            }
            SelectionAction::Continue
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.row_count > 0 {
                state.selected = state.selected.saturating_sub(1);
            }
            SelectionAction::Continue
        }
        KeyCode::Enter => SelectionAction::Submit,
        KeyCode::Esc | KeyCode::Char('q') => SelectionAction::Cancel,
        _ => SelectionAction::Continue,
    }
}

fn run_provider_selector_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    providers: &[ProviderConfig],
) -> Result<String, AppError> {
    let mut state = SelectorState::new(providers.len());

    loop {
        terminal.draw(|frame| {
            let areas =
                Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());
            let table = build_provider_picker_table(providers, areas[0].width, Some(state.selected));
            let mut table_state = TableState::default();
            table_state.select(Some(state.selected));
            frame.render_stateful_widget(table, areas[0], &mut table_state);
            frame.render_widget(
                Paragraph::new(Line::from("j/k • ↑/↓ move   enter confirm   q cancel"))
                    .style(Style::default().fg(ui_palette().help_fg)),
                areas[1],
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if !should_process_key_event(key) {
                continue;
            }
            match handle_key_code(&mut state, key.code) {
                SelectionAction::Continue => {}
                SelectionAction::Submit => return Ok(providers[state.selected].name.clone()),
                SelectionAction::Cancel => {
                    return Err(AppError::Message("selection cancelled".to_string()));
                }
            }
        }
    }
}

fn run_template_selector_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    choices: &[TemplateChoice],
) -> Result<TemplateChoice, AppError> {
    let mut state = SelectorState::new(choices.len());

    loop {
        terminal.draw(|frame| {
            let areas =
                Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());
            let table = build_template_picker_table(choices, areas[0].width, Some(state.selected));
            let mut table_state = TableState::default();
            table_state.select(Some(state.selected));
            frame.render_stateful_widget(table, areas[0], &mut table_state);
            frame.render_widget(
                Paragraph::new(Line::from("j/k • ↑/↓ move   enter confirm   q cancel"))
                    .style(Style::default().fg(ui_palette().help_fg)),
                areas[1],
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if !should_process_key_event(key) {
                continue;
            }
            match handle_key_code(&mut state, key.code) {
                SelectionAction::Continue => {}
                SelectionAction::Submit => return Ok(choices[state.selected].clone()),
                SelectionAction::Cancel => {
                    return Err(AppError::Message("selection cancelled".to_string()));
                }
            }
        }
    }
}

fn build_provider_picker_table(
    providers: &[ProviderConfig],
    width: u16,
    selected: Option<usize>,
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
            .style(Style::default().fg(ui_palette().body_fg))
    });

    Table::new(rows, picker_widths(width))
        .header(header)
        .block(picker_block("Providers"))
        .row_highlight_style(selected_row_style())
}

fn build_template_picker_table(
    choices: &[TemplateChoice],
    width: u16,
    selected: Option<usize>,
) -> Table<'static> {
    let header = Row::new(vec![Cell::from(""), Cell::from("Name"), Cell::from("Base URL")]).style(
        Style::default()
            .fg(ui_palette().header_fg)
            .add_modifier(Modifier::BOLD),
    );

    let rows = choices.iter().enumerate().map(|(index, choice)| match choice {
        TemplateChoice::BuiltIn(template) => {
            let marker = if selected == Some(index) { ">" } else { " " };

            Row::new(vec![
                Cell::from(marker),
                Cell::from(template.display_name.clone()),
                Cell::from(template.base_url.clone()),
            ])
                .style(Style::default().fg(ui_palette().body_fg))
        }
        TemplateChoice::Custom => {
            let marker = if selected == Some(index) { ">" } else { " " };

            Row::new(vec![
                Cell::from(marker),
                Cell::from("Custom").style(Style::default().fg(ui_palette().custom_accent)),
                Cell::from("Manual endpoint setup"),
            ])
            .style(Style::default().fg(ui_palette().body_fg))
        }
    });

    Table::new(rows, picker_widths(width))
        .header(header)
        .block(picker_block("Provider Templates"))
        .row_highlight_style(selected_row_style())
}

fn picker_widths(width: u16) -> [Constraint; 3] {
    if width >= 100 {
        [Constraint::Length(2), Constraint::Length(22), Constraint::Min(24)]
    } else {
        [Constraint::Length(2), Constraint::Length(16), Constraint::Min(18)]
    }
}

fn picker_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui_palette().border))
        .title(title)
        .title_style(
            Style::default()
                .fg(ui_palette().title)
                .add_modifier(Modifier::BOLD),
        )
}

fn selected_row_style() -> Style {
    Style::default()
        .fg(ui_palette().selected_fg)
        .bg(ui_palette().selected_bg)
        .add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::{build_template_picker_table, selected_row_style};
    use crate::provider_templates::{ProviderTemplate, TemplateChoice};
    use crate::ui::theme::ui_palette;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::{StatefulWidget, TableState};

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

    #[test]
    fn picker_paragraph_aligns_base_url_column_across_rows() {
        let area = Rect::new(0, 0, 80, 6);
        let mut buffer = Buffer::empty(area);
        let choices = vec![
            TemplateChoice::BuiltIn(ProviderTemplate {
                id: "ggboom".into(),
                display_name: "GGBoom".into(),
                base_url: "https://ai.qaq.al".into(),
                default_model: None,
                notes: None,
            }),
            TemplateChoice::BuiltIn(ProviderTemplate {
                id: "quickly".into(),
                display_name: "MuchLongerName".into(),
                base_url: "https://sub.jlypx.de".into(),
                default_model: None,
                notes: None,
            }),
        ];
        let table = build_template_picker_table(&choices, area.width, Some(0));
        let mut state = TableState::default();
        state.select(Some(0));
        StatefulWidget::render(table, area, &mut buffer, &mut state);
        let output = buffer_to_string(&buffer);
        let rows: Vec<_> = output
            .lines()
            .filter(|line| line.contains("https://ai.qaq.al") || line.contains("https://sub.jlypx.de"))
            .collect();

        let first = rows[0].find("https://").unwrap();
        let second = rows[1].find("https://").unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn selected_row_style_uses_palette_highlight() {
        let palette = ui_palette();
        let style = selected_row_style();
        assert_eq!(style.fg, Some(palette.selected_fg));
        assert_eq!(style.bg, Some(palette.selected_bg));
    }
}
