#![allow(unknown_lints)]
#![deny(
    clippy::expect_used,
    clippy::explicit_deref_methods,
    clippy::option_if_let_else,
    clippy::await_holding_lock,
    clippy::cloned_instead_of_copied,
    clippy::explicit_into_iter_loop,
    clippy::flat_map_option,
    clippy::fn_params_excessive_bools,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::large_types_passed_by_value,
    clippy::manual_ok_or,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::must_use_candidate,
    clippy::needless_for_each,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::redundant_else,
    clippy::semicolon_if_nothing_returned,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unnested_or_patterns,
    clippy::future_not_send,
    clippy::useless_let_if_seq,
    clippy::str_to_string,
    clippy::inherent_to_string,
    clippy::let_and_return,
    clippy::string_to_string,
    clippy::try_err,
    clippy::unused_async,
    clippy::missing_enforced_import_renames,
    clippy::nonstandard_macro_braces,
    clippy::rc_mutex,
    clippy::unwrap_or_else_default,
    clippy::manual_split_once,
    clippy::derivable_impls,
    clippy::needless_option_as_deref,
    clippy::iter_not_returning_iterator,
    clippy::same_name_method,
    clippy::manual_assert,
    clippy::non_send_fields_in_send_ty,
    clippy::equatable_if_let,
    bad_style,
    clashing_extern_declarations,
    dead_code,
    deprecated,
    explicit_outlives_requirements,
    improper_ctypes,
    invalid_value,
    missing_copy_implementations,
    missing_debug_implementations,
    mutable_transmutes,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    trivial_bounds,
    trivial_casts,
    trivial_numeric_casts,
    type_alias_bounds,
    unconditional_recursion,
    unreachable_pub,
    unsafe_code,
    unstable_features,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_import_braces,
    unused_parens,
    unused_qualifications,
    while_true,
    missing_docs
)]
#![allow(
    unused_attributes,
    clippy::derive_partial_eq_without_eq,
    clippy::box_default,
    missing_docs
)]

mod list;
mod process;
mod procfile;
mod terminal;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use bytes::Bytes;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use portable_pty::CommandBuilder;
use ratatui::{backend::Backend, widgets::Widget, Terminal};

use self::process::ProcessGroup;

fn shell_cmd<S: AsRef<str>>(cmd: S, options: &[&str], cwd: &Path) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(cmd.as_ref());
    for opt in options {
        cmd.arg(opt);
    }
    // cmd.arg("-c");
    // cmd.arg(shell_string.as_ref());
    cmd.cwd(cwd);
    cmd
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Configuration File
    #[arg(short, long, default_value = "Procfile")]
    config: PathBuf,
}

fn main() -> anyhow::Result<()> {
    // Build a tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");

    // Parse the command line arguments
    let args = Args::parse();

    // Run our main function
    rt.block_on(async_main(args))?;

    // Shutdown any lingering processes;
    rt.shutdown_background();
    Ok(())
}

async fn async_main(args: Args) -> anyhow::Result<()> {
    let config = tokio::fs::read_to_string(&args.config).await?;

    let procfile = procfile::parse(&config).expect("Failed parsing procfile");

    let mut terminal = terminal::setup_terminal()?;
    let size = terminal.size()?;
    let mut group = ProcessGroup::new(size.height, size.width);
    let cwd = std::env::current_dir().unwrap();

    for proc in procfile {
        group.add(proc.name, shell_cmd(proc.command, &proc.options, &cwd))?;
    }

    run(&mut terminal, group).await?;
    terminal::cleanup_terminal(terminal)?;

    Ok(())
}

async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    mut group: ProcessGroup,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| group.clone().render(f.area(), f.buffer_mut()))?;

        group = match handle_event(group).await {
            Ok(Some(group)) => group,
            Ok(None) => return Ok(()),
            Err(e) => {
                eprintln!("Error: {:?}", e);
                return Err(e);
            }
        }
    }
}

async fn handle_event(mut group: ProcessGroup) -> anyhow::Result<Option<ProcessGroup>> {
    // timeout if an event is not received within `Duration` so we don't block.
    if event::poll(Duration::from_millis(10))? {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('c') => {
                            if matches!(key.modifiers, KeyModifiers::CONTROL) {
                                return Ok(None);
                            }
                        }
                        KeyCode::Char('q') => return Ok(None),
                        KeyCode::Char(input) => {
                            group.handle_input(Bytes::from(input.to_string().into_bytes()))?
                        }
                        KeyCode::Backspace => {
                            group.handle_input(Bytes::from(vec![8]))?;
                        }
                        KeyCode::Enter => group.handle_input(Bytes::from(vec![b'\n']))?,
                        KeyCode::Left => group.prev(),
                        KeyCode::Right => group.next(),
                        KeyCode::Up => group.handle_input(Bytes::from(vec![27, 91, 65]))?,
                        KeyCode::Down => group.handle_input(Bytes::from(vec![27, 91, 66]))?,
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
            Event::Resize(cols, rows) => {
                group.resize(cols, rows);

                // parser.write().set_size(rows, cols);
            }
        }
    }
    Ok(Some(group))
}
