use std::sync::Arc;

use parking_lot::RwLock;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use tui_term::widget::PseudoTerminal;

use super::screen::ProcessScreen;

#[derive(Clone)]
pub(crate) struct ProcessWidget<'a> {
    name: &'a str,
    sized: bool,
    pub(crate) parser: Arc<RwLock<vt100::Parser>>,
}

impl<'a> ProcessWidget<'a> {
    pub(crate) fn new(process: &'a mut ProcessScreen) -> Self {
        let sized = process.sized;
        // the screen's parser gets resized on first render, so we modify our
        // screen to reflect that it's parser has been (or will imminently be) resized.
        (*process).sized = true;
        Self {
            name: &process.name,
            sized,
            parser: process.parser.clone(),
        }
    }
}

impl<'a> Widget for ProcessWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let [header, main] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)].as_ref())
            .areas(area);

        let p = Paragraph::new(self.name)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        p.render(header, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().add_modifier(Modifier::BOLD));

        if !self.sized {
            self.parser.write().set_size(main.height, main.width);
        }

        let parser = self.parser.read();
        let screen = parser.screen();
        let pseudo_term = PseudoTerminal::new(screen).block(block);

        pseudo_term.render(main, buf);
    }
}
