use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::components::alert::Alert;
use crate::components::chat::Chat;
use crate::components::login::Login;
use crate::components::navigation::Navigation;
use crate::components::recent_chat::RecentChat;
use crate::{
    action::Action,
    components::Component,
    config::Config,
    tui::{Event, Tui},
};

pub(crate) static SHOULD_QUIT: LazyLock<Arc<Mutex<ShouldQuit>>> =
    LazyLock::new(|| Arc::new(Mutex::new(ShouldQuit { should_quit: false })));

pub(crate) struct ShouldQuit {
    pub(crate) should_quit: bool,
}

impl ShouldQuit {
    pub(crate) fn set_quit(&mut self, should_quit: bool) {
        self.should_quit = should_quit;
    }
}

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_suspend: bool,
    mode: ModeHolderLock,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Login,
    RecentChat,
    Chat,
    Contact,
    Setting,
    Alert,
}

#[derive(Default)]
pub struct ModeHolder {
    pub mode: Mode,
}

impl ModeHolder {
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
}

#[derive(Clone)]
pub struct ModeHolderLock(pub Arc<Mutex<ModeHolder>>);

impl ModeHolderLock {
    pub fn set_mode(&self, mode: Mode) {
        self.0.lock().unwrap().set_mode(mode);
    }

    pub fn get_mode(&self) -> Mode {
        self.0.lock().unwrap().mode
    }
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let mode_holder = ModeHolderLock(Arc::new(Mutex::new(ModeHolder::default())));
        let login = Login::new(mode_holder.clone());
        let navigation = Navigation::new(mode_holder.clone());
        let recent_chat = RecentChat::new(mode_holder.clone());
        let alert = Alert::new(mode_holder.clone());
        let chat = Chat::new(mode_holder.clone());
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![
                Box::new(login),
                Box::new(navigation),
                Box::new(recent_chat),
                Box::new(alert),
                Box::new(chat),
            ],
            should_suspend: false,
            config: Config::new()?,
            mode: mode_holder.clone(),
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if SHOULD_QUIT.lock().unwrap().should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(keymap) = self.config.keybindings.get(&self.mode.get_mode()) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
            _ => {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => {
                    SHOULD_QUIT.lock().unwrap().set_quit(true);
                }
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                _ => {}
            }
            for component in self.components.iter_mut() {
                if let Some(action) = component.update(action.clone())? {
                    self.action_tx.send(action)?
                };
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if let Err(err) = component.draw(frame, frame.area()) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(format!("Failed to draw: {:?}", err)));
                }
            }
        })?;
        Ok(())
    }
}
