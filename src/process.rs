use std::{
    io::{BufWriter, Read, Write},
    sync::Arc,
};

use bytes::Bytes;
use parking_lot::RwLock;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem, SlavePty};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use tokio::{
    sync::mpsc::{channel, error::SendError, Receiver, Sender},
    task,
};
use tui_term::widget::PseudoTerminal;

use crate::list::List;

#[derive(Clone)]
pub struct ProcessGroup {
    blocks: Arc<RwLock<List<ProcessContainer>>>,
    sender: Sender<Bytes>,
    rows: u16,
    cols: u16,
}

impl ProcessGroup {
    pub fn next(&self) {
        let mut blocks = self.blocks.write();
        blocks.next();
    }

    pub fn prev(&self) {
        let mut blocks = self.blocks.write();
        blocks.prev();
    }

    pub fn parser(&self) -> Arc<RwLock<vt100::Parser>> {
        let blocks = self.blocks.read();
        blocks.selected().parser.clone()
    }

    pub fn new(rows: u16, cols: u16) -> Self {
        let (tx, mut rx) = channel::<Bytes>(32);
        let blocks = Arc::new(RwLock::new(List::<ProcessContainer>::new()));
        let _input_task = {
            let blocks = blocks.clone();
            tokio::spawn(async move {
                while let Some(bytes) = rx.recv().await {
                    let blocks = blocks.read();
                    let selected = blocks.selected();
                    selected.handle_input(bytes);
                }
            })
        };

        Self {
            blocks,
            sender: tx,
            rows,
            cols,
        }
    }

    pub fn add(&mut self, cmd: CommandBuilder) -> anyhow::Result<()> {
        let block = ProcessContainer::from_something(cmd, self.rows, self.cols)?;
        let mut blocks = self.blocks.write();
        blocks.add(block);
        Ok(())
    }

    pub async fn handle_input(&self, input: Bytes) -> Result<(), SendError<Bytes>> {
        self.sender.send(input).await
    }
}

impl Widget for ProcessGroup {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let [main, footer] = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(100), Constraint::Min(1)].as_ref())
            .areas(area);
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().add_modifier(Modifier::BOLD));

        let parser = self.parser();
        let lock = parser.read();

        // let screen = self.parser().read();
        let screen = lock.screen();
        let pseudo_term = PseudoTerminal::new(screen).block(block);

        pseudo_term.render(main, buf);

        // f.render_widget(pseudo_term, chunks[0]);
        let explanation = "Press q to exit".to_string();
        let explanation = Paragraph::new(explanation)
            .style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED))
            .alignment(Alignment::Center);
        explanation.render(footer, buf);
        // f.render_widget(explanation, chunks[1]);
    }
}

pub struct ProcessContainer {
    sender: Sender<Bytes>,
    #[allow(unused)]
    child_task: task::JoinHandle<()>,
    #[allow(unused)]
    output_task: task::JoinHandle<()>,
    #[allow(unused)]
    writer_task: task::JoinHandle<()>,
    pub parser: Arc<RwLock<vt100::Parser>>,
}

impl ProcessContainer {
    pub fn handle_input(&self, input: Bytes) {
        let sender = self.sender.clone();
        tokio::spawn(async move { sender.send(input).await });
    }

    pub fn from_something(cmd: CommandBuilder, rows: u16, cols: u16) -> anyhow::Result<Self> {
        let pty_system = NativePtySystem::default();
        let parser = Arc::new(RwLock::new(vt100::Parser::new(rows, cols, 0)));

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Wait for the child to complete
        let child_task = task::spawn(Self::command_runner(cmd, pair.slave));

        let reader = pair.master.try_clone_reader()?;

        let output_task = task::spawn(Self::output_reader(reader, parser.clone()));

        let (tx, rx) = channel::<Bytes>(32);

        let writer = BufWriter::new(pair.master.take_writer().unwrap());

        let writer_task = tokio::spawn(Self::output_writer(rx, writer, pair.master));

        Ok(Self {
            sender: tx,
            parser,
            child_task,
            output_task,
            writer_task,
        })
    }

    async fn command_runner(cmd: CommandBuilder, pty: Box<dyn SlavePty + Send>) {
        let mut child = pty.spawn_command(cmd).unwrap();
        let _child_exit_status = child.wait().unwrap();
        drop(pty);
    }

    async fn output_writer(
        mut rx: Receiver<Bytes>,
        mut writer: BufWriter<Box<dyn Write + Send>>,
        pty: Box<dyn MasterPty + Send>,
    ) {
        while let Some(bytes) = rx.recv().await {
            println!("here");
            if let Err(e) = writer.write_all(&bytes) {
                println!("oh no: {:?}", e);
                break;
            };
            if let Err(e) = writer.flush() {
                println!("oh no2: {:?}", e);
                break;
            }
        }
        drop(pty);
    }

    async fn output_reader(mut reader: Box<dyn Read + Send>, parser: Arc<RwLock<vt100::Parser>>) {
        // Consume the output from the child
        // Can't read the full buffer, since that would wait for EOF
        let mut buf = [0u8; 8192];
        let mut processed_buf = Vec::new();
        loop {
            let size = reader.read(&mut buf).unwrap();
            if size <= 0 {
                break;
            }
            processed_buf.extend_from_slice(&buf[..size]);
            parser.write().process(&processed_buf);

            // Clear the processed portion of the buffer
            processed_buf.clear();
        }
    }
}
