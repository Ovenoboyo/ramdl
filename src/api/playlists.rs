use super::artwork::Artwork;
use super::music_videos::MusicVideos;
use super::play_parameters::PlayParameters;
use super::songs::Songs;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Playlists {
    /// The identifier for the playlist.
    pub id: String,
    /// This value is always playlists.
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    /// The relative location for the playlist resource.
    pub href: String,
    /// The attributes for the playlist.
    pub attributes: Attributes,
    /// The relationships for the playlist.
    pub relationships: Option<Relationships>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attributes {
    /// The playlist artwork.
    pub artwork: Option<Artwork>,
    /// The display name of the curator.
    pub curator_name: Option<String>,
    /// A description of the playlist.
    pub description: Option<DescriptionAttribute>,
    /// Indicates whether the playlist represents a popularity chart.
    #[serde(default)]
    pub is_chart: bool,
    /// The date the playlist was last modified.
    #[serde(default)]
    pub last_modified_date: Option<String>,
    /// The localized name of the playlist.
    pub name: String,
    /// The type of playlist.
    #[serde(default)]
    pub playlist_type: Option<String>,
    /// The value map may be used to initiate playback of available tracks in the playlist.
    pub play_params: Option<PlayParameters>,
    /// The URL for sharing the playlist in Apple Music.
    pub url: String,
    /// (Extended) The resource types that are present in the tracks of the playlists.  
    /// Possible Values: music-videos, songs
    #[serde(default)]
    pub track_types: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DescriptionAttribute {
    /// An abbreviated description to show inline or when the content appears alongside other content.
    pub short: Option<String>,
    /// A description to show when the content is prominently displayed.
    pub standard: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Relationships {
    /// The songs and music videos included in the playlist. By default, tracks includes objects
    pub tracks: Option<Tracks>,
    // /// The curator that created the playlist. By default, curator includes identifiers only.
    // pub curator: Option<Curator>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    /// A relative location for the relationship.
    pub href: Option<String>,
    /// A relative cursor to fetch the next paginated collection of resources in the relationship if more exist.
    pub next: Option<String>,
    /// The data for the included tracks.
    pub data: Vec<TrackData>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TrackData {
    Songs(Songs),
    MusicVideos(MusicVideos),
    Other(serde_json::Value),
}

// #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
// pub struct Curator {
//     /// A relative location for the relationship.
//     pub href: Option<String>,
//     /// A relative cursor to fetch the next paginated collection of resources in the relationship if more exist.
//     pub next: Option<String>,
//     /// The curator for the playlist.
//     pub data: Vec<CuratorData>,
// }
