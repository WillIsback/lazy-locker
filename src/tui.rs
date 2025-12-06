use anyhow::Result;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{Stdout, stdout};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialise le terminal : active le Raw Mode et bascule sur l'écran alternatif
pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = stdout();

    // Try to enable keyboard enhancement for better compatibility
    // This is optional and may fail on some terminals
    let _ = execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );

    // Fallback if keyboard enhancement failed
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;
    Ok(terminal)
}

/// Restaure le terminal : quitte l'écran alternatif et désactive le Raw Mode
pub fn restore() -> Result<()> {
    let mut stdout = stdout();
    // Try to pop keyboard enhancement (ignore errors)
    let _ = execute!(stdout, PopKeyboardEnhancementFlags);
    execute!(stdout, DisableMouseCapture, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
