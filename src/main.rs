#![deny(unused)]
mod list;
mod process;

use std::{
    io::{self},
    path::Path,
    time::Duration,
};

use bytes::Bytes;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::ResetColor,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use portable_pty::CommandBuilder;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::Widget,
    Terminal,
};

use self::process::ProcessGroup;

async fn shell_cmd(shell_string: &str, cwd: &Path) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(shell_string);
    cmd.cwd(cwd);
    cmd
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, ResetColor)?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let cwd = std::env::current_dir().unwrap();
    let cmd = shell_cmd("find .", &cwd).await;
    let cmd2 = shell_cmd("ls -alf .", &cwd).await;
    let size = terminal.size()?;

    let mut group = ProcessGroup::new(size.height, size.width);
    group.add(cmd)?;
    group.add(cmd2)?;

    run(&mut terminal, group).await?;

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run<B: Backend>(terminal: &mut Terminal<B>, group: ProcessGroup) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| group.clone().render(f.area(), f.buffer_mut()))?;

        match handle_event(&group).await {
            Ok(true) => break Ok(()),
            Ok(false) => {}
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    }
}

async fn handle_event(group: &ProcessGroup) -> anyhow::Result<bool> {
    if event::poll(Duration::from_millis(10))? {
        // It's guaranteed that the `read()` won't block when the `poll()`
        // function returns `true`
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(true),
                        KeyCode::Char(input) => {
                            group
                                .handle_input(Bytes::from(input.to_string().into_bytes()))
                                .await?
                        }
                        KeyCode::Backspace => {
                            group.handle_input(Bytes::from(vec![8])).await?;
                        }
                        KeyCode::Enter => group.handle_input(Bytes::from(vec![b'\n'])).await?,
                        KeyCode::Left => group.prev(),
                        KeyCode::Right => group.next(),
                        // KeyCode::Left => sender.send(Bytes::from(vec![27, 91, 68])).await?,
                        // KeyCode::Right => sender.send(Bytes::from(vec![27, 91, 67])).await?,
                        KeyCode::Up => group.handle_input(Bytes::from(vec![27, 91, 65])).await?,
                        KeyCode::Down => group.handle_input(Bytes::from(vec![27, 91, 66])).await?,
                        KeyCode::Home => todo!(),
                        KeyCode::End => todo!(),
                        KeyCode::PageUp => todo!(),
                        KeyCode::PageDown => todo!(),
                        KeyCode::Tab => todo!(),
                        KeyCode::BackTab => todo!(),
                        KeyCode::Delete => todo!(),
                        KeyCode::Insert => todo!(),
                        KeyCode::F(_) => todo!(),
                        KeyCode::Null => todo!(),
                        KeyCode::Esc => todo!(),
                        KeyCode::CapsLock => todo!(),
                        KeyCode::ScrollLock => todo!(),
                        KeyCode::NumLock => todo!(),
                        KeyCode::PrintScreen => todo!(),
                        KeyCode::Pause => todo!(),
                        KeyCode::Menu => todo!(),
                        KeyCode::KeypadBegin => todo!(),
                        KeyCode::Media(_) => todo!(),
                        KeyCode::Modifier(_) => todo!(),
                    }
                }
            }
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(_) => {}
            Event::Paste(_) => todo!(),
            Event::Resize(_cols, _rows) => {
                todo!()
                // parser.write().set_size(rows, cols);
            }
        }
    }
    Ok(false)
}
