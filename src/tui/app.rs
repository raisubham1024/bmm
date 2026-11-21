use super::commands::Command;
use super::common::*;
use super::handle::handle_command;
use super::message::{Message, get_event_handling_msg};
use super::model::*;
use super::update::update;
use super::view::view;
use crate::persistence::SearchScope;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use sqlx::{Pool, Sqlite};
use std::io::Error as IOError;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};

const EVENT_POLL_DURATION_MS: u64 = 16;

#[derive(thiserror::Error, Debug)]
pub enum AppTuiError {
    #[error("couldn't initialize bmm's TUI: {0}")]
    InitializeTerminal(IOError),
    #[error("couldn't determine terminal size: {0}")]
    DetermineTerminalSize(IOError),
    #[error("couldn't restore terminal to its original state: {0}")]
    RestoreTerminal(IOError),
    #[error("couldn't send a message to internal async queue: {0}")]
    SendMsg(#[from] TrySendError<Message>),
    #[error("couldn't draw a TUI frame: {0}")]
    DrawFrame(IOError),
    #[error("couldn't poll for internal events: {0}")]
    PollForEvents(IOError),
    #[error("couldn't read internal event: {0}")]
    ReadEvent(IOError),
}

pub async fn run_tui(
    pool: &Pool<Sqlite>,
    context: TuiContext,
    db_name: String,
) -> Result<(), AppTuiError> {
    let mut tui = AppTui::new(pool, context, db_name)?;
    tui.run().await?;

    Ok(())
}

impl AppTuiError {
    pub fn code(&self) -> u16 {
        match self {
            AppTuiError::DetermineTerminalSize(_) => 5000,
            AppTuiError::InitializeTerminal(_) => 5001,
            AppTuiError::RestoreTerminal(_) => 5002,
            AppTuiError::SendMsg(_) => 5003,
            AppTuiError::DrawFrame(_) => 5004,
            AppTuiError::PollForEvents(_) => 5005,
            AppTuiError::ReadEvent(_) => 5006,
        }
    }
}

/// Terminal features bmm switches on while it runs, and makes sure are
/// switched off again however bmm exits:
/// - mouse reporting (clicks/taps and the scroll wheel reach bmm);
/// - the "keyboard enhancement" protocol, when the terminal supports it,
///   which is the only way a terminal can report the Super (Windows) key,
///   needed for Alt+Super+1..5 in the database list.
struct TerminalExtras {
    keyboard_enhancement: bool,
}

impl TerminalExtras {
    fn enable() -> Self {
        use ratatui::crossterm::event::{
            EnableMouseCapture, KeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
        };

        // a terminal that can't do these just keeps working with the
        // keyboard the way it always did
        let _ = ratatui::crossterm::execute!(std::io::stdout(), EnableMouseCapture);

        let keyboard_enhancement =
            ratatui::crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false)
                && ratatui::crossterm::execute!(
                    std::io::stdout(),
                    PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
                )
                .is_ok();

        Self {
            keyboard_enhancement,
        }
    }

    fn disable(&mut self) {
        let _ = ratatui::crossterm::execute!(
            std::io::stdout(),
            ratatui::crossterm::event::DisableMouseCapture
        );

        if self.keyboard_enhancement {
            self.keyboard_enhancement = false;
            let _ = ratatui::crossterm::execute!(
                std::io::stdout(),
                ratatui::crossterm::event::PopKeyboardEnhancementFlags
            );
        }
    }
}

impl Drop for TerminalExtras {
    fn drop(&mut self) {
        self.disable();
    }
}

struct AppTui {
    terminal_extras: TerminalExtras,
    pub(super) terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    pub(super) event_tx: Sender<Message>,
    pub(super) event_rx: Receiver<Message>,
    pub(super) model: Model,
    pub(super) initial_commands: Vec<Command>,
}

impl AppTui {
    pub fn new(
        pool: &Pool<Sqlite>,
        context: TuiContext,
        db_name: String,
    ) -> Result<Self, AppTuiError> {
        let terminal = ratatui::try_init().map_err(AppTuiError::InitializeTerminal)?;
        let terminal_extras = TerminalExtras::enable();
        let (event_tx, event_rx) = mpsc::channel(10);
        let mut initial_commands = Vec::new();

        let (width, height) =
            ratatui::crossterm::terminal::size().map_err(AppTuiError::DetermineTerminalSize)?;

        let terminal_dimensions = TerminalDimensions { width, height };

        match &context {
            TuiContext::Initial => {}
            TuiContext::Search(q) => {
                initial_commands.push(Command::SearchBookmarks(q.clone(), SearchScope::All));
            }
            TuiContext::Tags => {}
        }
        // Tag-name autocomplete in the add/edit bookmark screen reads
        // from `all_tag_items`, which needs this to have run - regardless
        // of which context bmm started in, not just `TuiContext::Tags`.
        // (Previously this only ran for `TuiContext::Tags`, which meant
        // suggestions silently never showed up unless the user had
        // visited the Tags List view - 't' - at least once first.)
        initial_commands.push(Command::FetchTags);
        initial_commands.push(Command::FetchStarredUris);

        let model = Model::default(pool, context, terminal_dimensions, db_name);

        Ok(Self {
            terminal_extras,
            terminal,
            event_tx,
            event_rx,
            model,
            initial_commands,
        })
    }

    pub async fn run(&mut self) -> Result<(), AppTuiError> {
        let _ = self.terminal.clear();

        for cmd in &self.initial_commands {
            handle_command(&self.model.pool, cmd.clone(), self.event_tx.clone()).await;
        }

        // first render
        self.model.render_counter += 1;
        self.terminal
            .draw(|f| view(&mut self.model, f))
            .map_err(AppTuiError::DrawFrame)?;

        loop {
            tokio::select! {
                Some(message) = self.event_rx.recv() => {
                    let cmds = update(&mut self.model, message);

                    if self.model.running_state == RunningState::Done {
                        self.exit().map_err(AppTuiError::RestoreTerminal)?;
                        return Ok(());
                    }

                        self.model.render_counter += 1;
                        self.terminal.draw(|f| view(&mut self.model, f)).map_err(AppTuiError::DrawFrame)?;

                    for cmd in cmds {
                        handle_command(&self.model.pool, cmd, self.event_tx.clone()).await;
                    }
                }

                Ok(ready) = tokio::task::spawn_blocking(|| ratatui::crossterm::event::poll(Duration::from_millis(EVENT_POLL_DURATION_MS))) => {
                    match ready {
                        Ok(true) => {
                            let event = normalize_event(
                                ratatui::crossterm::event::read().map_err(AppTuiError::ReadEvent)?,
                            );
                            self.model.event_counter += 1;
                            if let Some(handling_msg) = get_event_handling_msg(&self.model, event) {
                                self.event_tx.try_send(handling_msg)?;
                            }
                        }
                        Ok(false) => {
                            // Terminals don't report the Alt key being
                            // released, so once a pause follows the last
                            // Alt+<digit> typed in the Database List, the
                            // number typed so far is applied.
                            if self.model.alt_number_expired() {
                                let _ = self.event_tx.try_send(Message::AltDigitTimeout);
                            }
                            continue;
                        }
                        Err(e) => {
                                return Err(AppTuiError::PollForEvents(e));
                        }
                    }
                }
            }
        }
    }

    fn exit(&mut self) -> Result<(), IOError> {
        // mouse reporting / enhanced keys go off before leaving the
        // alternate screen
        self.terminal_extras.disable();
        ratatui::try_restore()
    }
}

/// With the keyboard enhancement protocol on, a terminal reports
/// Alt+Shift+b as the *unshifted* key `b` plus a Shift modifier, where
/// without it the key arrives as `B`. bmm matches the capital letter (eg.
/// Alt+B must not be taken for Alt+b), so make both forms look the same.
fn normalize_event(event: ratatui::crossterm::event::Event) -> ratatui::crossterm::event::Event {
    use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};

    match event {
        Event::Key(mut key)
            if key.modifiers.contains(KeyModifiers::SHIFT)
                && matches!(key.code, KeyCode::Char(c) if c.is_ascii_lowercase()) =>
        {
            if let KeyCode::Char(c) = key.code {
                key.code = KeyCode::Char(c.to_ascii_uppercase());
            }
            Event::Key(key)
        }
        other => other,
    }
}
