use age::x25519::{Identity, Recipient};
use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};
use std::time::Duration;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::info;

use super::comms::Comms;
use crate::common::{Auth, ClientMsg, Note, ServerMsg};

pub fn run(
    comms: &mut Comms,
    key: Identity,
    recipient: Recipient,
    shutdown_tx: Sender<()>,
    shutdown_rx: Receiver<()>,
) -> Result<()> {
    info!("🖥️ Started TUI");
    let terminal = ratatui::init();
    let app = App::new(comms, key, recipient, shutdown_tx, shutdown_rx);
    let app_res = app.run(terminal);
    ratatui::restore();
    info!("🖥️ Stopped TUI");
    app_res
}

const POLL_DURATION_MILLIS: u64 = 10;

/// App holds the state of the application
struct App<'a> {
    /// Communication with server
    comms: &'a mut Comms,
    /// Current private key
    priv_key: Identity,
    /// Current public key
    pub_key: Recipient,
    /// Whether or not we've succesfully authenticated
    authenticated: bool,
    /// Current recipient pubkey we are chatting with
    recipient: Recipient,
    /// History of recorded notes (chat messages)
    notes: Vec<Note>,
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Channels to coordinate shutdowns with the rest of the program
    shutdown_tx: Sender<()>,
    shutdown_rx: Receiver<()>,
}

impl<'a> App<'a> {
    fn new(
        comms: &'a mut Comms,
        key: Identity,
        recipient: Recipient,
        shutdown_tx: Sender<()>,
        shutdown_rx: Receiver<()>,
    ) -> Self {
        Self {
            comms,
            pub_key: key.to_public(),
            priv_key: key,
            authenticated: false,
            recipient,
            notes: Vec::new(),
            input: String::new(),
            character_index: 0,
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Run the main app loop
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Authenticate
        info!(
            "✍️ Attempting to authenticate to server as {}",
            self.pub_key
        );
        self.comms
            .try_send_msg(ClientMsg::AuthReq(Auth::new(self.pub_key.to_string())))?;

        loop {
            // Shutdown
            if self.shutdown_rx.try_recv().is_ok() {
                info!("⛔ Received shutdown signal");
                return Ok(());
            };

            // Handle new messages
            while let Ok(msg) = self.comms.try_recv_msg() {
                self.handle_msg(msg)?;
            }

            // Don't do anything else unless authenticated
            if !self.authenticated {
                continue;
            }

            // Draw the TUI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle keypresses
            self.handle_keypresses()?;
        }
    }

    /// Handle incoming message from the server
    fn handle_msg(&mut self, msg: ServerMsg) -> Result<()> {
        match msg {
            ServerMsg::AuthSecret(auth) => {
                info!(
                    "✍️ Decrypting secret {} for pubkey {} to authenticate to the server",
                    auth.ciphertext, auth.pub_key
                );
                // TODO: what if pub_key in auth is different than self.pub_key?
                // TODO: this is not secure, as the server can have the client decrypt arbitrary secrets
                let plaintext =
                    String::from_utf8(age::decrypt(&self.priv_key, auth.ciphertext.as_bytes())?)?;
                let auth_plaintext = Auth {
                    pub_key: auth.pub_key,
                    plaintext,
                    ciphertext: auth.ciphertext,
                };
                self.comms
                    .try_send_msg(ClientMsg::AuthPlaintext(auth_plaintext))?;
                Ok(())
            }
            ServerMsg::AuthGranted(auth) => {
                info!(
                    "✍️ Successfully authenticated to server as {}",
                    auth.pub_key
                );
                self.authenticated = true;
                Ok(())
            }
            ServerMsg::AuthDenied(auth) => {
                info!(
                    "✍️ Failed authenticating to server as {}, shutting down",
                    auth.pub_key
                );
                self.shutdown_tx.send(())?;
                Ok(())
            }
            ServerMsg::RecNote(note) => {
                info!("✉️ Received new note");
                self.notes.push(note);
                Ok(())
            }
        }
    }

    /// Handle keypresses, using poll so we don't block forever waiting
    fn handle_keypresses(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(POLL_DURATION_MILLIS))? {
            let Event::Key(key) = event::read()? else {
                return Ok(());
            };
            if key.kind != KeyEventKind::Press {
                return Ok(());
            };

            match key.code {
                KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                    self.shutdown_tx.send(())?;
                    return Ok(());
                }
                KeyCode::Enter => self.submit_note()?,
                KeyCode::Char(to_insert) => self.enter_char(to_insert),
                KeyCode::Backspace => self.delete_char(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                _ => {}
            }
        }

        Ok(())
    }

    /// Send a note when the user presses enter
    fn submit_note(&mut self) -> Result<()> {
        let note = Note::encrypt_new(&self.pub_key, &self.recipient, self.input.clone())?;
        self.comms.try_send_msg(ClientMsg::SendNote(note))?;

        self.input.clear();
        self.reset_cursor();
        Ok(())
    }

    /// Draw the TUI
    fn draw(&self, frame: &mut Frame) {
        let true_black = Color::Rgb(0, 0, 0);
        let true_white = Color::Rgb(255, 255, 255);

        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]);
        let [notes_area, input_area] = vertical.areas(frame.area());

        let notes: Vec<ListItem> = self
            .notes
            .iter()
            .map(|n| {
                let content = Line::from(Span::raw(
                    self.render_note(n)
                        .unwrap_or("<error rendering note>".to_string()),
                ));
                ListItem::new(content)
            })
            .collect();
        let notes = List::new(notes)
            .style(Style::default().fg(true_white).bg(true_black))
            .block(Block::bordered().title("Messages"));
        frame.render_widget(notes, notes_area);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(true_white).bg(true_black))
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);

        frame.set_cursor_position(Position::new(
            input_area.x + self.character_index as u16 + 1,
            input_area.y + 1,
        ));
    }

    /// Render a note as a String for display in the TUI
    fn render_note(&self, note: &Note) -> Result<String> {
        let local_time = note.timestamp.with_timezone(&Local);
        let timestamp_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();
        Ok(format!(
            "[{timestamp_str}] {}: {}",
            note.from,
            note.decrypt_content(&self.priv_key)?
        ))
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }
}
