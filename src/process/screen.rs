use std::{
    io::{BufWriter, Read, Write},
    sync::Arc,
};

use bytes::Bytes;
use parking_lot::RwLock;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem, SlavePty};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task,
};

#[derive(Clone)]
pub(crate) struct ProcessScreen {
    pub(crate) name: String,
    sender: Option<Sender<Bytes>>,
    tasks: Option<Arc<Vec<task::JoinHandle<()>>>>,
    pub(crate) sized: bool,
    pub(crate) parser: Arc<RwLock<vt100::Parser>>,
}

impl ProcessScreen {
    pub(crate) fn new(
        name: String,
        cmd: CommandBuilder,
        rows: u16,
        cols: u16,
    ) -> anyhow::Result<Self> {
        let pty_system = NativePtySystem::default();

        let parser = Arc::new(RwLock::new(vt100::Parser::new(rows, cols, 0)));

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let child_task = task::spawn(async move {
            Self::command_runner(cmd, pair.slave);
        });

        let reader = pair.master.try_clone_reader()?;

        let output_task = task::spawn(Self::output_reader(reader, parser.clone()));

        let (tx, rx) = channel::<Bytes>(32);

        let writer = BufWriter::new(pair.master.take_writer().unwrap());

        let writer_task = tokio::spawn(Self::output_writer(rx, writer, pair.master));

        Ok(Self {
            name,
            sender: Some(tx),
            parser,
            tasks: Some(Arc::new(vec![child_task, writer_task, output_task])),
            sized: false,
        })
    }

    pub(crate) fn recalculate_size(&mut self) {
        self.sized = false;
    }

    pub(crate) fn handle_input(&self, input: Bytes) {
        let sender = self.sender.clone();
        tokio::spawn(async move { sender.clone().unwrap().send(input).await });
    }

    fn command_runner(cmd: CommandBuilder, pty: Box<dyn SlavePty + Send>) {
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
            if let Err(e) = writer.write_all(&bytes) {
                println!("error writing to writer: {:?}", e);
                break;
            };
            if let Err(e) = writer.flush() {
                println!("error flushing writer: {:?}", e);
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

impl Drop for ProcessScreen {
    fn drop(&mut self) {
        if let Some(tasks) = self.tasks.take() {
            for task in tasks.iter() {
                task.abort();
            }
        }
        self.sender.take();
    }
}
