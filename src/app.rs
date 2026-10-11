use std::collections::HashSet;
use std::io::Stdout;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use ratatui::Terminal;
use rspotify::model::{FullTrack, PlaylistId, SimplifiedPlaylist};
use rspotify::AuthCodePkceSpotify;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Login,
    Browser,
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pane {
    Playlists,
    Tracks,
}

#[allow(dead_code)]
pub enum Action {
    AuthSuccess(Box<AuthCodePkceSpotify>, String), // spotify client + user display name
    PlaylistsLoaded(Vec<SimplifiedPlaylist>),
    TracksLoaded(Vec<FullTrack>, HashSet<String>), // tracks + set of track IDs
    SearchResults(Vec<FullTrack>),
    TrackAdded(String),   // track ID that was added
    TrackRemoved(String), // track ID that was removed
    PlaylistCreated,
    SetStatus(String),
    Error(String),
}

pub struct App {
    pub screen: Screen,
    pub spotify: Option<AuthCodePkceSpotify>,
    pub user_name: String,

    // Playlist browser
    pub playlists: Vec<SimplifiedPlaylist>,
    pub playlist_state: ListState,
    pub tracks: Vec<FullTrack>,
    pub track_state: ListState,
    pub active_pane: Pane,

    // Search
    pub search_query: String,
    pub search_results: Vec<FullTrack>,
    pub search_state: ListState,
    pub playlist_track_ids: HashSet<String>, // track IDs in current playlist

    // Async
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub action_rx: mpsc::UnboundedReceiver<Action>,

    // Status
    pub status_msg: String,
    pub loading: bool,

    // New playlist input
    pub new_playlist_input: Option<String>,

    // Search debounce
    pub search_dirty: bool,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            screen: Screen::Login,
            spotify: None,
            user_name: String::new(),
            playlists: Vec::new(),
            playlist_state: ListState::default(),
            tracks: Vec::new(),
            track_state: ListState::default(),
            active_pane: Pane::Playlists,
            search_query: String::new(),
            search_results: Vec::new(),
            search_state: ListState::default(),
            playlist_track_ids: HashSet::new(),
            action_tx,
            action_rx,
            status_msg: "Starting...".to_string(),
            loading: true,
            new_playlist_input: None,
            search_dirty: false,
        })
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> anyhow::Result<()> {
        // Spawn authentication in background
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match crate::auth::authenticate().await {
                Ok((spotify, name)) => {
                    let _ = tx.send(Action::AuthSuccess(Box::new(spotify), name));
                }
                Err(e) => {
                    let _ = tx.send(Action::Error(format!("Authentication failed: {}", e)));
                }
            }
        });

        let mut event_stream = EventStream::new();
        let mut search_debounce: Option<tokio::time::Instant> = None;

        loop {
            terminal.draw(|f| crate::ui::draw(f, self))?;

            let debounce_future = async {
                match search_debounce {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending().await,
                }
            };

            tokio::select! {
                Some(Ok(event)) = event_stream.next() => {
                    match self.handle_event(event, &mut search_debounce) {
                        Ok(true) => break, // quit requested
                        Ok(false) => {}
                        Err(e) => self.status_msg = format!("Error: {}", e),
                    }
                }
                Some(action) = self.action_rx.recv() => {
                    self.handle_action(action);
                }
                _ = debounce_future => {
                    self.fire_search();
                    search_debounce = None;
                }
            }
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: Event,
        search_debounce: &mut Option<tokio::time::Instant>,
    ) -> anyhow::Result<bool> {
        if let Event::Key(key) = event {
            // Only handle key press events (ignore release and repeat)
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }

            // Ctrl+C always quits
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                return Ok(true);
            }

            // Handle new playlist name input modal
            if self.new_playlist_input.is_some() {
                match key.code {
                    KeyCode::Enter => {
                        if let Some(name) = self.new_playlist_input.take() {
                            let trimmed = name.trim().to_string();
                            if !trimmed.is_empty() {
                                self.create_playlist(trimmed);
                            }
                        }
                    }
                    KeyCode::Esc => {
                        self.new_playlist_input = None;
                        self.status_msg = "Playlist creation cancelled".to_string();
                    }
                    KeyCode::Char(c) => {
                        if let Some(ref mut input) = self.new_playlist_input {
                            input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(ref mut input) = self.new_playlist_input {
                            input.pop();
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }

            match self.screen {
                Screen::Login => {
                    // 'q' quits during login screen
                    if key.code == KeyCode::Char('q') {
                        return Ok(true);
                    }
                }
                Screen::Browser => match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Char('/') | KeyCode::Char('s') => {
                        self.screen = Screen::Search;
                        self.search_query.clear();
                        self.search_results.clear();
                        self.search_state = ListState::default();
                    }
                    KeyCode::Char('n') => {
                        self.new_playlist_input = Some(String::new());
                        self.status_msg =
                            "Enter playlist name (Enter to confirm, Esc to cancel)".to_string();
                    }
                    KeyCode::Tab => {
                        self.active_pane = match self.active_pane {
                            Pane::Playlists => Pane::Tracks,
                            Pane::Tracks => Pane::Playlists,
                        };
                    }
                    KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                    KeyCode::Enter => {
                        if self.active_pane == Pane::Playlists {
                            self.select_playlist();
                        }
                    }
                    KeyCode::Char('d') => {
                        if self.active_pane == Pane::Tracks {
                            self.remove_selected_track();
                        }
                    }
                    KeyCode::Char('r') => {
                        self.refresh_playlists();
                    }
                    _ => {}
                },
                Screen::Search => {
                    match key.code {
                        KeyCode::Esc => {
                            self.screen = Screen::Browser;
                            // Refresh tracks for current playlist
                            self.refresh_current_tracks();
                        }
                        KeyCode::Up => {
                            let len = self.search_results.len();
                            if len > 0 {
                                let i = self.search_state.selected().unwrap_or(0);
                                let new_i = if i == 0 { len - 1 } else { i - 1 };
                                self.search_state.select(Some(new_i));
                            }
                        }
                        KeyCode::Down => {
                            let len = self.search_results.len();
                            if len > 0 {
                                let i = self.search_state.selected().unwrap_or(0);
                                let new_i = if i >= len - 1 { 0 } else { i + 1 };
                                self.search_state.select(Some(new_i));
                            }
                        }
                        KeyCode::Enter => {
                            self.toggle_search_track();
                        }
                        KeyCode::Backspace => {
                            self.search_query.pop();
                            self.search_dirty = true;
                            *search_debounce = Some(
                                tokio::time::Instant::now()
                                    + Duration::from_millis(crate::config::SEARCH_DEBOUNCE_MS),
                            );
                        }
                        KeyCode::Char(c) => {
                            self.search_query.push(c);
                            self.search_dirty = true;
                            *search_debounce = Some(
                                tokio::time::Instant::now()
                                    + Duration::from_millis(crate::config::SEARCH_DEBOUNCE_MS),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(false)
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::AuthSuccess(spotify, name) => {
                let spotify = *spotify;
                self.spotify = Some(spotify.clone());
                self.user_name = name;
                self.screen = Screen::Browser;
                self.status_msg = format!("Logged in as {}", self.user_name);
                self.loading = true;
                crate::spotify::fetch_playlists(&spotify, &self.action_tx);
            }
            Action::PlaylistsLoaded(playlists) => {
                self.playlists = playlists;
                if !self.playlists.is_empty() {
                    self.playlist_state.select(Some(0));
                    self.select_playlist();
                }
                self.loading = false;
            }
            Action::TracksLoaded(tracks, ids) => {
                self.tracks = tracks;
                self.playlist_track_ids = ids;
                if !self.tracks.is_empty() {
                    self.track_state.select(Some(0));
                } else {
                    self.track_state.select(None);
                }
                self.loading = false;
            }
            Action::SearchResults(results) => {
                self.search_results = results;
                if !self.search_results.is_empty() {
                    self.search_state.select(Some(0));
                } else {
                    self.search_state.select(None);
                }
                self.loading = false;
            }
            Action::TrackAdded(id) => {
                self.playlist_track_ids.insert(id);
                self.status_msg = "Track added!".to_string();
            }
            Action::TrackRemoved(id) => {
                self.playlist_track_ids.remove(&id);
                self.tracks.retain(|t| {
                    if let Some(ref tid) = t.id {
                        tid.to_string() != id
                    } else {
                        true
                    }
                });
                if let Some(selected) = self.track_state.selected() {
                    if !self.tracks.is_empty() && selected >= self.tracks.len() {
                        self.track_state.select(Some(self.tracks.len() - 1));
                    }
                }
                self.status_msg = "Track removed!".to_string();
            }
            Action::PlaylistCreated => {
                self.status_msg = "Playlist created!".to_string();
                self.refresh_playlists();
            }
            Action::SetStatus(msg) => {
                self.status_msg = msg;
            }
            Action::Error(msg) => {
                self.status_msg = format!("Error: {}", msg);
                self.loading = false;
            }
        }
    }

    fn move_up(&mut self) {
        match self.active_pane {
            Pane::Playlists => {
                let len = self.playlists.len();
                if len > 0 {
                    let i = self.playlist_state.selected().unwrap_or(0);
                    let new_i = if i == 0 { len - 1 } else { i - 1 };
                    self.playlist_state.select(Some(new_i));
                }
            }
            Pane::Tracks => {
                let len = self.tracks.len();
                if len > 0 {
                    let i = self.track_state.selected().unwrap_or(0);
                    let new_i = if i == 0 { len - 1 } else { i - 1 };
                    self.track_state.select(Some(new_i));
                }
            }
        }
    }

    fn move_down(&mut self) {
        match self.active_pane {
            Pane::Playlists => {
                let len = self.playlists.len();
                if len > 0 {
                    let i = self.playlist_state.selected().unwrap_or(0);
                    let new_i = if i >= len - 1 { 0 } else { i + 1 };
                    self.playlist_state.select(Some(new_i));
                }
            }
            Pane::Tracks => {
                let len = self.tracks.len();
                if len > 0 {
                    let i = self.track_state.selected().unwrap_or(0);
                    let new_i = if i >= len - 1 { 0 } else { i + 1 };
                    self.track_state.select(Some(new_i));
                }
            }
        }
    }

    fn selected_playlist_id(&self) -> Option<PlaylistId<'static>> {
        let idx = self.playlist_state.selected()?;
        let playlist = self.playlists.get(idx)?;
        Some(playlist.id.clone_static())
    }

    fn select_playlist(&mut self) {
        if let Some(playlist_id) = self.selected_playlist_id() {
            self.loading = true;
            self.status_msg = "Loading tracks...".to_string();
            if let Some(ref spotify) = self.spotify {
                crate::spotify::fetch_tracks(spotify, &playlist_id, &self.action_tx);
            }
        }
    }

    fn remove_selected_track(&mut self) {
        if let (Some(playlist_id), Some(track_idx)) =
            (self.selected_playlist_id(), self.track_state.selected())
        {
            if let Some(track) = self.tracks.get(track_idx) {
                if let Some(ref track_id) = track.id {
                    if let Some(ref spotify) = self.spotify {
                        crate::spotify::remove_track(
                            spotify,
                            &playlist_id,
                            track_id,
                            &self.action_tx,
                        );
                    }
                }
            }
        }
    }

    fn toggle_search_track(&mut self) {
        if let Some(idx) = self.search_state.selected() {
            if let Some(track) = self.search_results.get(idx) {
                if let Some(ref track_id) = track.id {
                    if let Some(playlist_id) = self.selected_playlist_id() {
                        let id_str = track_id.to_string();
                        if let Some(ref spotify) = self.spotify {
                            if self.playlist_track_ids.contains(&id_str) {
                                crate::spotify::remove_track(
                                    spotify,
                                    &playlist_id,
                                    track_id,
                                    &self.action_tx,
                                );
                            } else {
                                crate::spotify::add_track(
                                    spotify,
                                    &playlist_id,
                                    track_id,
                                    &self.action_tx,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn fire_search(&mut self) {
        let trimmed = self.search_query.trim();
        if trimmed.len() >= 2 {
            if let Some(ref spotify) = self.spotify {
                self.loading = true;
                crate::spotify::search_tracks(spotify, trimmed, &self.action_tx);
            }
        }
        self.search_dirty = false;
    }

    fn refresh_playlists(&mut self) {
        if let Some(ref spotify) = self.spotify {
            self.loading = true;
            crate::spotify::fetch_playlists(spotify, &self.action_tx);
        }
    }

    fn refresh_current_tracks(&mut self) {
        if let Some(playlist_id) = self.selected_playlist_id() {
            if let Some(ref spotify) = self.spotify {
                crate::spotify::fetch_tracks(spotify, &playlist_id, &self.action_tx);
            }
        }
    }

    fn create_playlist(&mut self, name: String) {
        if let Some(ref spotify) = self.spotify {
            crate::spotify::create_playlist(spotify, &name, &self.action_tx);
            self.status_msg = format!("Creating playlist '{}'...", name);
        }
    }
}
