use crate::app::Action;
use futures::StreamExt;
use rspotify::model::{PlayableId, PlayableItem, PlaylistId, SearchResult, SearchType, TrackId};
use rspotify::prelude::*;
use rspotify::AuthCodePkceSpotify;
use std::collections::HashSet;
use tokio::sync::mpsc;

pub fn fetch_playlists(spotify: &AuthCodePkceSpotify, tx: &mpsc::UnboundedSender<Action>) {
    let spotify = spotify.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let mut playlists = Vec::new();
        let mut stream = spotify.current_user_playlists();
        while let Some(result) = stream.next().await {
            match result {
                Ok(playlist) => playlists.push(playlist),
                Err(e) => {
                    let _ = tx.send(Action::Error(format!("Failed to load playlists: {}", e)));
                    return;
                }
            }
        }
        let _ = tx.send(Action::PlaylistsLoaded(playlists));
    });
}

pub fn fetch_tracks(
    spotify: &AuthCodePkceSpotify,
    playlist_id: &PlaylistId<'_>,
    tx: &mpsc::UnboundedSender<Action>,
) {
    let spotify = spotify.clone();
    let playlist_id = playlist_id.clone_static();
    let tx = tx.clone();
    tokio::spawn(async move {
        let mut tracks = Vec::new();
        let mut track_ids = HashSet::new();

        let mut stream = spotify.playlist_items(playlist_id, None, None);
        while let Some(result) = stream.next().await {
            match result {
                Ok(item) => {
                    if let Some(PlayableItem::Track(track)) = item.item {
                        if let Some(ref id) = track.id {
                            track_ids.insert(id.to_string());
                        }
                        tracks.push(track);
                    }
                }
                Err(e) => {
                    let _ = tx.send(Action::Error(format!("Failed to load tracks: {}", e)));
                    return;
                }
            }
        }
        let _ = tx.send(Action::TracksLoaded(tracks, track_ids));
    });
}

pub fn search_tracks(
    spotify: &AuthCodePkceSpotify,
    query: &str,
    tx: &mpsc::UnboundedSender<Action>,
) {
    let spotify = spotify.clone();
    let query = query.trim().to_string();
    let tx = tx.clone();
    tokio::spawn(async move {
        match spotify
            .search(&query, SearchType::Track, None, None, Some(10), None)
            .await
        {
            Ok(result) => {
                if let SearchResult::Tracks(page) = result {
                    let _ = tx.send(Action::SearchResults(page.items));
                }
            }
            Err(e) => {
                let _ = tx.send(Action::Error(format!("Search failed: {}", e)));
            }
        }
    });
}

pub fn add_track(
    spotify: &AuthCodePkceSpotify,
    playlist_id: &PlaylistId<'_>,
    track_id: &TrackId<'_>,
    tx: &mpsc::UnboundedSender<Action>,
) {
    let spotify = spotify.clone();
    let playlist_id = playlist_id.clone_static();
    let track_id = track_id.clone_static();
    let tx = tx.clone();
    tokio::spawn(async move {
        let items = [PlayableId::Track(track_id.clone())];
        match spotify.playlist_add_items(playlist_id, items, None).await {
            Ok(_) => {
                let _ = tx.send(Action::TrackAdded(track_id.to_string()));
            }
            Err(e) => {
                let _ = tx.send(Action::Error(format!("Failed to add track: {}", e)));
            }
        }
    });
}

pub fn remove_track(
    spotify: &AuthCodePkceSpotify,
    playlist_id: &PlaylistId<'_>,
    track_id: &TrackId<'_>,
    tx: &mpsc::UnboundedSender<Action>,
) {
    let spotify = spotify.clone();
    let playlist_id = playlist_id.clone_static();
    let track_id = track_id.clone_static();
    let tx = tx.clone();
    tokio::spawn(async move {
        let items = [PlayableId::Track(track_id.clone())];
        match spotify
            .playlist_remove_all_occurrences_of_items(playlist_id, items, None)
            .await
        {
            Ok(_) => {
                let _ = tx.send(Action::TrackRemoved(track_id.to_string()));
            }
            Err(e) => {
                let _ = tx.send(Action::Error(format!("Failed to remove track: {}", e)));
            }
        }
    });
}

pub fn create_playlist(
    spotify: &AuthCodePkceSpotify,
    name: &str,
    tx: &mpsc::UnboundedSender<Action>,
) {
    let spotify = spotify.clone();
    let name = name.to_string();
    let tx = tx.clone();
    tokio::spawn(async move {
        match spotify.current_user().await {
            Ok(user) => {
                match spotify
                    .user_playlist_create(user.id, &name, Some(false), Some(false), None)
                    .await
                {
                    Ok(_) => {
                        let _ = tx.send(Action::PlaylistCreated);
                    }
                    Err(e) => {
                        let _ = tx.send(Action::Error(format!("Failed to create playlist: {}", e)));
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Action::Error(format!("Failed to get user: {}", e)));
            }
        }
    });
}
