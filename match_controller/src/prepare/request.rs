use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::graphql::post_graphql;
use crate::request::MatchRequest;
use crate::settings::Settings;

pub struct DownloadLinks {
    pub map: String,
    pub bot1_zip: String,
    pub bot1_data: Option<String>,
    pub bot2_zip: String,
    pub bot2_data: Option<String>,
}

const MATCH_QUERY: &str = r#"
query($id: ID!) {
  node(id: $id) {
    ... on MatchType {
      id
      databaseId
      map {
        name
        downloadLink
      }
      participant1 {
        name
        gameDisplayId
        playsRace { databaseId }
        botZipUrl
        botDataUrl
      }
      participant2 {
        name
        gameDisplayId
        playsRace { databaseId }
        botZipUrl
        botDataUrl
      }
    }
  }
}
"#;

#[derive(Debug, Deserialize)]
struct MatchResponse {
    data: Option<MatchData>,
}

#[derive(Debug, Deserialize)]
struct MatchData {
    node: Option<MatchInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchInfo {
    id: String,
    database_id: u32,
    map: MapInfo,
    participant1: ParticipantInfo,
    participant2: ParticipantInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapInfo {
    name: String,
    download_link: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BotRaceInfo {
    database_id: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParticipantInfo {
    name: String,
    game_display_id: String,
    plays_race: BotRaceInfo,
    bot_zip_url: String,
    bot_data_url: Option<String>,
}

pub async fn fetch_match_request(settings: &Settings) -> anyhow::Result<(MatchRequest, DownloadLinks)> {
    let body = serde_json::json!({ "query": MATCH_QUERY, "variables": { "id": settings.match_graph_id } });
    let text = post_graphql(settings, "request", body).await?;
    let parsed: MatchResponse = serde_json::from_str(&text).context("Failed to parse node match response")?;

    let m = parsed
        .data
        .ok_or_else(|| anyhow!("node match response has no data"))?
        .node
        .ok_or_else(|| anyhow!("node match response returned no match for id {}", settings.match_graph_id))?;

    if m.id != settings.match_graph_id {
        return Err(anyhow!("match id mismatch: expected {}, got {}", settings.match_graph_id, m.id));
    }
    if m.database_id != settings.match_display_id {
        return Err(anyhow!("match display id mismatch: expected {}, got {}", settings.match_display_id, m.database_id));
    }

    let match_request = MatchRequest {
        match_id: m.database_id,
        map_name: format!("{}.SC2Map", m.map.name),
        player_1_id: m.participant1.game_display_id.clone(),
        player_1_name: m.participant1.name.clone(),
        player_1_race: m.participant1.plays_race.database_id,
        player_2_id: m.participant2.game_display_id.clone(),
        player_2_name: m.participant2.name.clone(),
        player_2_race: m.participant2.plays_race.database_id,
    };
    let download_links = DownloadLinks {
        map: m.map.download_link,
        bot1_zip: m.participant1.bot_zip_url,
        bot1_data: m.participant1.bot_data_url,
        bot2_zip: m.participant2.bot_zip_url,
        bot2_data: m.participant2.bot_data_url,
    };
    Ok((match_request, download_links))
}
