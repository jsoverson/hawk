use std::sync::Arc;

use bytes::Bytes;
use parking_lot::RwLock;
use portable_pty::CommandBuilder;
use ratatui::{prelude::*, widgets::Paragraph};
use tokio::sync::mpsc::error::SendError;

use crate::list::List;

use super::{screen::ProcessScreen, widget::ProcessWidget};

#[derive(Clone)]
pub(crate) struct ProcessGroup {
    blocks: Arc<RwLock<List<ProcessScreen>>>,
    rows: u16,
    cols: u16,
    sized: bool,
}

impl ProcessGroup {
    pub(crate) fn next(&self) {
        let mut blocks = self.blocks.write();
        blocks.next();
    }

    pub(crate) fn prev(&self) {
        let mut blocks = self.blocks.write();
        blocks.prev();
    }

    pub(crate) fn new(rows: u16, cols: u16) -> Self {
        let blocks = Arc::new(RwLock::new(List::<ProcessScreen>::new()));
        Self {
            blocks,
            rows,
            cols,
            sized: false,
        }
    }

    pub(crate) fn add(&mut self, name: &str, cmd: CommandBuilder) -> anyhow::Result<()> {
        let block = ProcessScreen::new(name.to_owned(), cmd, self.rows, self.cols)?;
        let mut blocks = self.blocks.write();
        blocks.add(block);
        Ok(())
    }

    pub(crate) fn handle_input(&self, input: Bytes) -> Result<(), SendError<Bytes>> {
        let blocks = self.blocks.write();
        blocks.focused().handle_input(input);
        Ok(())
    }

    pub(crate) fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
        let mut blocks = self.blocks.write();
        for block in blocks.iter_mut() {
            block.recalculate_size();
        }
        self.sized = false;
    }
}

impl Widget for ProcessGroup {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let [main, footer] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)].as_ref())
            .areas(area);

        let [left, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
            .areas(main);

        let mut blocks = self.blocks.write();
        ProcessWidget::new(blocks.get_mut(0).unwrap()).render(left, buf);
        ProcessWidget::new(blocks.get_mut(1).unwrap()).render(right, buf);

        let explanation = "Press q to exit".to_string();
        let explanation = Paragraph::new(explanation)
            .style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED))
            .alignment(Alignment::Center);
        explanation.render(footer, buf);
    }
}
