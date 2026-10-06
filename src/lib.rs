//! Rust Apple Music Downloader.

pub mod api;
pub mod decrypter;
pub mod error;
pub mod stream_info;

use crate::api::*;
use crate::error::Error;
use crate::error::Result;
use base64::Engine;
use fancy_regex::Regex;
use serde_json::json;

use lyrics::Lyrics;
use search;
use songs::Songs;
use stream_info::StreamInfo;

/// <https://beta.music.apple.com>
pub const APPLE_MUSIC_HOMEPAGE_URL: &str = "https://beta.music.apple.com";
/// <https://amp-api.music.apple.com>
pub const AMP_API_URL: &str = "https://amp-api.music.apple.com";
/// <https://play.itunes.apple.com/WebObjects/MZPlay.woa/wa/webPlayback>
pub const WEBPLAYBACK_API_URL: &str =
    "https://play.itunes.apple.com/WebObjects/MZPlay.woa/wa/webPlayback";
/// <https://play.itunes.apple.com/WebObjects/MZPlay.woa/wa/acquireWebPlaybackLicense>
pub const LICENSE_API_URL: &str =
    "https://play.itunes.apple.com/WebObjects/MZPlay.woa/wa/acquireWebPlaybackLicense";

/// The Apple Music downloader struct.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppleMusicDownloader {
    media_user_token: String,
    store_front: String,
    language: String,
    headers: reqwest::header::HeaderMap,
    client: reqwest::Client,
    device: widevine::Device,
}

impl Default for AppleMusicDownloader {
    fn default() -> Self {
        let device =
            widevine::Device::read_wvd(include_bytes!("../device/device.wvd") as &[u8]).unwrap();
        AppleMusicDownloader {
            media_user_token: "".to_string(),
            store_front: "us".to_string(),
            language: "en-US".to_string(),
            headers: reqwest::header::HeaderMap::new(),
            client: reqwest::Client::new(),
            device,
        }
    }
}

impl AppleMusicDownloader {
    /// Creates a new `AppleMusicDownloader` instance with the provided media user token, store front, and language.
    /// # Examples
    /// ```rust
    /// # use ramdl::AppleMusicDownloader;
    /// let apple_music_downloader = AppleMusicDownloader::new("Asc+xxx", "us", "en-US", "eyJhxxx");
    /// ```
    pub fn new(media_user_token: &str, store_front: &str, language: &str, session: &str) -> Self {
        if media_user_token.is_empty() {
            panic!("Media user token is empty");
        }
        if store_front.is_empty() {
            panic!("Store front is empty");
        }
        if language.is_empty() {
            panic!("Language is empty");
        }
        if session.is_empty() {
            panic!("Session is empty");
        }
        let mut apple_music_downloader = AppleMusicDownloader {
            media_user_token: media_user_token.to_string(),
            store_front: store_front.to_string(),
            language: language.to_string(),
            ..Default::default()
        };
        apple_music_downloader.init_headers().unwrap();
        apple_music_downloader.headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {session}").parse().unwrap(),
        );
        apple_music_downloader.create_client().unwrap();
        apple_music_downloader
    }

    /// Creates a new `AppleMusicDownloader` instance with the provided media user token. This function will automatically get the store front and language from the Apple Music API.
    /// # Examples
    /// ```rust
    /// # use ramdl::AppleMusicDownloader;
    /// let apple_music_downloader = AppleMusicDownloader::new_with_media_user_token("Asc+xxx");
    /// ```
    pub async fn new_with_media_user_token(media_user_token: &str) -> Result<Self> {
        let mut apple_music_downloader = AppleMusicDownloader {
            media_user_token: media_user_token.to_string(),
            ..Default::default()
        };
        apple_music_downloader.init_session().await?;
        apple_music_downloader.create_client()?;
        apple_music_downloader.init_headers()?;
        apple_music_downloader.create_client()?;
        if !apple_music_downloader.media_user_token.is_empty() {
            let _ = apple_music_downloader.init_storefront_language().await;
        }
        Ok(apple_music_downloader)
    }

    // Initializes the Apple Music session.
    async fn init_session(&mut self) -> Result<()> {
        let home_page = self
            .client
            .get(APPLE_MUSIC_HOMEPAGE_URL)
            .send()
            .await?
            .text()
            .await?;

        let js_path = if let Some(pos) = home_page.find("/assets/index~") {
            let rest = &home_page[pos..];
            if let Some(end) = rest.find(".js") {
                &rest[..end + 3]
            } else {
                return Err(Error::Init("Parsing index.js path error".to_string()));
            }
        } else {
            return Err(Error::Init("index~.js not found on home page".to_string()));
        };

        let js_res = self
            .client
            .get(format!("{APPLE_MUSIC_HOMEPAGE_URL}{js_path}"))
            .send()
            .await?
            .text()
            .await?;

        let mut token = None;
        let mut from = 0usize;
        while let Some(at) = js_res[from..].find("eyJ") {
            let start = from + at;
            let len = js_res[start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                .count();
            let candidate = &js_res[start..start + len];
            if candidate.matches('.').count() == 2 && candidate.len() >= 100 {
                token = Some(candidate);
                break;
            }
            from = start + 3;
        }

        let token = token.ok_or_else(|| Error::Init("Failed to find JWT token in JS bundle".to_string()))?;

        self.headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        Ok(())
    }

    // Initializes the request headers.
    fn init_headers(&mut self) -> Result<()> {
        self.headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:95.0) Gecko/20100101 Firefox/95.0"
                .parse()
                .unwrap(),
        );
        self.headers
            .insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());
        self.headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        self.headers.insert(
            reqwest::header::HeaderName::from_static("media-user-token"),
            self.media_user_token.parse().unwrap(),
        );
        self.headers.insert(
            reqwest::header::ORIGIN,
            APPLE_MUSIC_HOMEPAGE_URL.parse().unwrap(),
        );
        Ok(())
    }

    // Initializes the storefront and language.
    async fn init_storefront_language(&mut self) -> Result<()> {
        let res = self
            .client
            .get(format!("{AMP_API_URL}/v1/me/storefront"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        self.store_front = res["data"][0]["id"].as_str().unwrap().to_string();
        self.language = res["data"][0]["attributes"]["defaultLanguageTag"]
            .as_str()
            .unwrap()
            .to_string();
        Ok(())
    }

    // Creates a reqwest client with the apple_music_downloader.headers.
    fn create_client(&mut self) -> Result<()> {
        self.client = reqwest::Client::builder()
            .default_headers(self.headers.clone())
            .build()?;
        Ok(())
    }

    /// Gets the song information.
    pub async fn get_songs(&self, song_id: &str) -> Result<Songs> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/songs/{song_id}?include=albums&extend=extendedAssetUrls",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let song: Songs = serde_json::from_value(res["data"][0].clone())?;
        Ok(song)
    }

    /// Gets the lyrics information.
    pub async fn get_lyrics(&self, song_id: &str) -> Result<Vec<Option<Lyrics>>> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/songs/{song_id}?include=lyrics,syllable-lyrics&extend=extendedAssetUrls",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let lyrics: Option<Lyrics> =
            serde_json::from_value(res["data"][0]["relationships"]["lyrics"].clone()).ok();
        let syllable_lyrics: Option<Lyrics> =
            serde_json::from_value(res["data"][0]["relationships"]["syllable-lyrics"].clone()).ok();
        Ok(vec![lyrics, syllable_lyrics])
    }

    /// Searches for songs, albums, artists, and playlists.
    pub async fn search(&self, query: &str) -> Result<search::SearchResults> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/search?term={query}&types=songs,albums,artists,playlists&limit=25&offset=0",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let songs: Vec<search::Song> =
            serde_json::from_value(res["results"]["songs"]["data"].clone()).unwrap_or_default();
        let albums: Vec<search::Album> =
            serde_json::from_value(res["results"]["albums"]["data"].clone()).unwrap_or_default();
        let artists: Vec<search::Artist> =
            serde_json::from_value(res["results"]["artists"]["data"].clone()).unwrap_or_default();
        let playlists: Vec<search::Playlist> =
            serde_json::from_value(res["results"]["playlists"]["data"].clone()).unwrap_or_default();
        Ok(search::SearchResults {
            songs,
            albums,
            artists,
            playlists,
        })
    }

    /// Gets the Widevine license.
    pub async fn get_widevine_license(
        &self,
        track_id: &str,
        track_uri: &str,
        challenge: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let challenge_str = base64::engine::general_purpose::STANDARD.encode(&challenge);
        let response = self
            .client
            .post(LICENSE_API_URL)
            .json(&serde_json::json!({
                "challenge": challenge_str,
                "key-system": "com.widevine.alpha",
                "uri": track_uri,
                "adamId": track_id,
                "isLibrary": false,
                "user-initiated": true,
            }))
            .send()
            .await?;

        if response.status().is_success() {
            let response_dict: serde_json::Value = response.json().await?;
            if let Some(widevine_license) = response_dict.get("license") {
                let license = base64::engine::general_purpose::STANDARD
                    .decode(widevine_license.as_str().unwrap())?;
                return Ok(license);
            }
        }

        Err(Error::Init("Failed to get Widevine license".to_string()))
    }

    /// Gets the WebPlayback information.
    pub async fn get_webplayback(&self, track_id: &str) -> Result<webplayback::WebPlayBack> {
        let response = self
            .client
            .post(WEBPLAYBACK_API_URL)
            .body(
                json!({
                    "salableAdamId": track_id,
                    "language": self.language,
                })
                .to_string(),
            )
            .send()
            .await?
            .json::<webplayback::WebPlayBack>()
            .await?;
        Ok(response)
    }

    /// Gets the decryptioin key.
    pub async fn get_decryption_key(
        &self,
        stream_info: &StreamInfo,
        track_id: &str,
    ) -> Result<String> {
        let cdm = widevine::Cdm::new(self.device.clone());
        let decryption_key = decrypter::get_decrypt_key(&cdm, &stream_info.pssh, track_id, self)
            .await?;
        Ok(decryption_key)
    }

    /// Gets the album information.
    pub async fn get_album(&self, album_id: &str) -> Result<albums::Albums> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/albums/{album_id}",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let album: albums::Albums = serde_json::from_value(res["data"][0].clone())?;
        Ok(album)
    }

    /// Gets the album tracks.
    pub async fn get_album_tracks(&self, album_id: &str) -> Result<Vec<songs::Songs>> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/albums/{album_id}/tracks?limit=100",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let tracks: Vec<songs::Songs> =
            serde_json::from_value(res["data"].clone()).unwrap_or_default();
        Ok(tracks)
    }

    /// Gets the playlist information.
    pub async fn get_playlist(&self, playlist_id: &str) -> Result<playlists::Playlists> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/playlists/{playlist_id}",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let playlist: playlists::Playlists = serde_json::from_value(res["data"][0].clone())?;
        Ok(playlist)
    }

    /// Gets the playlist tracks.
    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<songs::Songs>> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/playlists/{playlist_id}/tracks?limit=100",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let tracks: Vec<songs::Songs> =
            serde_json::from_value(res["data"].clone()).unwrap_or_default();
        Ok(tracks)
    }

    /// Gets artist top songs.
    pub async fn get_artist_top_songs(&self, artist_id: &str) -> Result<Vec<songs::Songs>> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/artists/{artist_id}/view/top-songs?limit=50",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let songs: Vec<songs::Songs> =
            serde_json::from_value(res["data"].clone()).unwrap_or_default();
        Ok(songs)
    }

    /// Gets catalog song charts (recommendations).
    pub async fn get_charts(&self) -> Result<Vec<songs::Songs>> {
        let store_front = self.store_front.clone();
        let res = self
            .client
            .get(format!(
                "{AMP_API_URL}/v1/catalog/{store_front}/charts?types=songs&limit=25",
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let songs: Vec<songs::Songs> =
            serde_json::from_value(res["results"]["songs"][0]["data"].clone()).unwrap_or_default();
        Ok(songs)
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn store_front(&self) -> &str {
        &self.store_front
    }

    pub fn media_user_token(&self) -> &str {
        &self.media_user_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stream_info::StreamInfo;

    #[tokio::test]
    async fn get_decrypt_key() {
        let media_user_token = std::env::var("MEDIA_USER_TOKEN").unwrap();
        let apple_music_downloader =
            AppleMusicDownloader::new_with_media_user_token(&media_user_token)
                .await
                .unwrap();
        let webplayback = apple_music_downloader
            .get_webplayback("1753050648")
            .await
            .unwrap();
        let stream_info = StreamInfo::new_with_webplayback(&webplayback)
            .await
            .unwrap();
        let decryption_key = apple_music_downloader
            .get_decryption_key(&stream_info, "1753050648")
            .await
            .unwrap();

        assert_eq!(decryption_key.len(), 32);
    }
}
