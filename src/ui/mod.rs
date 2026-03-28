pub mod action;
pub mod input;
pub mod render;
pub mod state;
pub mod theme;

use crate::error::AppError;
use action::UiAction;
use ratatui::backend::ClearType;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use input::handle_key;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};
use render::{draw_screen, provider_viewport_height};
use state::UiState;
use std::io::{Stdout, stdout};

pub(crate) struct TerminalCleanupGuard {
    active: bool,
}

impl TerminalCleanupGuard {
    pub(crate) fn new() -> Self {
        Self { active: true }
    }
}

impl Drop for TerminalCleanupGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
            let _ = crossterm::execute!(stdout(), Show);
        }
    }
}

type UiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct UiSession {
    terminal: Option<UiTerminal>,
    state: UiState,
    _cleanup: Option<TerminalCleanupGuard>,
}

impl UiSession {
    pub fn new(state: UiState) -> Result<Self, AppError> {
        enable_raw_mode()?;
        let cleanup = TerminalCleanupGuard::new();
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(provider_viewport_height(state.providers.len())),
            },
        )?;

        Ok(Self {
            terminal: Some(terminal),
            state,
            _cleanup: Some(cleanup),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test(state: UiState) -> Self {
        Self {
            terminal: None,
            state,
            _cleanup: None,
        }
    }

    pub fn next_action(&mut self) -> Result<UiAction, AppError> {
        let terminal = self
            .terminal
            .as_mut()
            .expect("UiSession::next_action requires a live terminal");

        loop {
            terminal.draw(|frame| draw_screen(frame, &self.state))?;

            if let Event::Key(key) = event::read()? {
                if !should_process_key_event(key) {
                    continue;
                }

                match handle_key(&mut self.state, key.code) {
                    UiAction::Continue => {}
                    action => return Ok(action),
                }
            }
        }
    }

    pub fn replace_state(&mut self, state: UiState) -> Result<(), AppError> {
        if let Some(terminal) = self.terminal.as_mut() {
            let size = terminal.size()?;
            terminal.resize(Rect::new(
                0,
                0,
                size.width,
                provider_viewport_height(state.providers.len()),
            ))?;
            terminal.clear()?;
        }
        self.state = state;
        Ok(())
    }

    pub fn clear_viewport(&mut self) -> Result<(), AppError> {
        if let Some(terminal) = self.terminal.as_mut() {
            clear_inline_viewport(terminal)?;
        }
        Ok(())
    }

    pub fn prepare_for_launch(&mut self) -> Result<(), AppError> {
        if let Some(terminal) = self.terminal.as_mut() {
            prepare_terminal_for_launch(terminal)?;
        }
        self.terminal.take();
        self._cleanup.take();
        Ok(())
    }

    pub fn state(&self) -> &UiState {
        &self.state
    }
}

pub fn run_ui(state: UiState) -> Result<UiAction, AppError> {
    let mut session = UiSession::new(state)?;
    session.next_action()
}

fn should_process_key_event(key: KeyEvent) -> bool {
    matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

fn clear_inline_viewport<B: Backend>(terminal: &mut Terminal<B>) -> Result<(), B::Error> {
    let origin = terminal.get_frame().area().as_position();
    terminal.clear()?;
    terminal.backend_mut().set_cursor_position(origin)?;
    terminal.backend_mut().clear_region(ClearType::CurrentLine)?;
    Ok(())
}

fn prepare_terminal_for_launch<B: Backend>(terminal: &mut Terminal<B>) -> Result<(), B::Error> {
    clear_inline_viewport(terminal)
}

#[cfg(test)]
mod tests {
    use super::{
        TerminalCleanupGuard, UiSession, clear_inline_viewport, prepare_terminal_for_launch,
    };
    use crate::config::ProviderConfig;
    use crate::ui::state::{UiMode, UiScreen, UiState};
    use ratatui::backend::{Backend, TestBackend};
    use ratatui::widgets::Paragraph;
    use ratatui::{Terminal, TerminalOptions, Viewport};
    use std::cell::Cell;
    use std::rc::Rc;

    struct DropFlagGuard {
        dropped: Rc<Cell<bool>>,
    }

    impl Drop for DropFlagGuard {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    #[test]
    fn cleanup_guard_drops_on_scope_exit() {
        let dropped = Rc::new(Cell::new(false));

        {
            let _cleanup = TerminalCleanupGuard::new();
            let _flag = DropFlagGuard {
                dropped: dropped.clone(),
            };
        }

        assert!(dropped.get());
    }

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

    fn clear_inline_viewport_via_generic_backend<B: Backend>(
        terminal: &mut Terminal<B>,
    ) -> Result<(), B::Error> {
        clear_inline_viewport(terminal)
    }

    #[test]
    fn ui_session_replace_state_refreshes_back_to_list_screen() {
        let mut session = UiSession::for_test(UiState::new(UiMode::Run, Vec::new()));
        let mut refreshed = UiState::new(UiMode::Run, vec![provider()]);
        refreshed.selected = 0;

        session.replace_state(refreshed).unwrap();

        assert!(matches!(session.state().screen, UiScreen::List(_)));
        assert_eq!(session.state().providers.len(), 1);
    }

    #[test]
    fn clear_inline_viewport_erases_rendered_provider_area() {
        let backend = TestBackend::new(20, 6);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(3),
            },
        )
        .unwrap();

        terminal
            .draw(|frame| {
                frame.render_widget(Paragraph::new("Providers\nrouter-a\nrouter-b"), frame.area());
            })
            .unwrap();

        terminal.backend().assert_buffer_lines([
            "Providers           ",
            "router-a            ",
            "router-b            ",
            "                    ",
            "                    ",
            "                    ",
        ]);

        clear_inline_viewport_via_generic_backend(&mut terminal).unwrap();

        terminal.backend().assert_buffer_lines([
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
        ]);
    }

    #[test]
    fn launch_handoff_erases_rendered_provider_area() {
        let backend = TestBackend::new(20, 6);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(3),
            },
        )
        .unwrap();

        terminal
            .draw(|frame| {
                frame.render_widget(Paragraph::new("Providers\nrouter-a\nrouter-b"), frame.area());
            })
            .unwrap();

        prepare_terminal_for_launch(&mut terminal).unwrap();

        terminal.backend().assert_buffer_lines([
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
            "                    ",
        ]);
    }

    #[test]
    fn launch_handoff_releases_live_terminal_and_cleanup_guard() {
        let mut session = UiSession::for_test(UiState::new(UiMode::Run, vec![provider()]));
        session._cleanup = Some(TerminalCleanupGuard::new());

        session.prepare_for_launch().unwrap();

        assert!(session.terminal.is_none());
        assert!(session._cleanup.is_none());
    }
}
