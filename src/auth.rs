use crate::config;
use rspotify::prelude::*;
use rspotify::{scopes, AuthCodePkceSpotify, Config, Credentials, OAuth, Token};
use std::fs;
use tiny_http::{Header, Response, Server};

pub async fn authenticate() -> anyhow::Result<(AuthCodePkceSpotify, String)> {
    let creds = Credentials::from_env().ok_or_else(|| {
        anyhow::anyhow!("Missing RSPOTIFY_CLIENT_ID. Set it as env var or in .env file")
    })?;

    let oauth = OAuth {
        redirect_uri: config::REDIRECT_URI.to_string(),
        scopes: scopes!(
            "playlist-read-private",
            "playlist-read-collaborative",
            "playlist-modify-public",
            "playlist-modify-private",
            "user-read-private"
        ),
        ..Default::default()
    };

    // Ensure config directory exists
    let config_dir = config::config_dir();
    fs::create_dir_all(&config_dir)?;

    let token_path = config::token_path();
    let rspotify_config = Config {
        token_cached: true,
        cache_path: token_path.clone(),
        ..Default::default()
    };

    let mut spotify = AuthCodePkceSpotify::with_config(creds, oauth, rspotify_config);

    // Try cached token
    if token_path.exists() {
        if let Ok(data) = fs::read_to_string(&token_path) {
            if let Ok(token) = serde_json::from_str::<Token>(&data) {
                if let Ok(mut guard) = spotify.token.lock().await {
                    *guard = Some(token);
                }
                // Try to use it (will auto-refresh if needed)
                match spotify.current_user().await {
                    Ok(user) => {
                        let name = user.display_name.unwrap_or_else(|| user.id.to_string());
                        // Update cache in case it was refreshed
                        if let Ok(guard) = spotify.token.lock().await {
                            if let Some(ref tok) = *guard {
                                let _ = fs::write(&token_path, serde_json::to_string_pretty(tok)?);
                            }
                        }
                        return Ok((spotify, name));
                    }
                    Err(_) => {
                        // Token invalid/expired and could not refresh, proceed to fresh login
                        if let Ok(mut guard) = spotify.token.lock().await {
                            *guard = None;
                        }
                    }
                }
            }
        }
    }

    // Fresh login via PKCE
    let url = spotify.get_authorize_url(None)?;
    open::that(&url)?;

    let code = tokio::task::spawn_blocking(capture_callback_code).await??;
    spotify.request_token(&code).await?;

    // Save token to cache
    if let Ok(guard) = spotify.token.lock().await {
        if let Some(ref tok) = *guard {
            let _ = fs::write(&token_path, serde_json::to_string_pretty(tok)?);
        }
    }

    let user = spotify.current_user().await?;
    let name = user.display_name.unwrap_or_else(|| user.id.to_string());

    Ok((spotify, name))
}

fn capture_callback_code() -> anyhow::Result<String> {
    let server = Server::http(format!("127.0.0.1:{}", config::CALLBACK_PORT))
        .map_err(|e| anyhow::anyhow!("Failed to start callback server: {}", e))?;

    let request = server
        .recv()
        .map_err(|e| anyhow::anyhow!("Failed to receive callback request: {}", e))?;
    let url = request.url().to_string();

    // Parse the code from the URL query params: /callback?code=AQD...&state=...
    let code = url
        .split('?')
        .nth(1)
        .and_then(|query| {
            query.split('&').find_map(|param| {
                let (key, value) = param.split_once('=')?;
                if key == "code" {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| anyhow::anyhow!("No authorization code in callback URL"))?;

    let response_html = "<html><body><h1>Authentication successful!</h1><p>You can close this tab and return to the terminal.</p></body></html>";
    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
        .map_err(|_| anyhow::anyhow!("Failed to build HTTP response header"))?;
    let response = Response::from_string(response_html).with_header(header);
    request
        .respond(response)
        .map_err(|e| anyhow::anyhow!("Failed to respond to callback request: {}", e))?;

    Ok(code)
}
