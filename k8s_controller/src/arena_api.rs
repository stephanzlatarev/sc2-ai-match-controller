use anyhow::{anyhow, Context};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GetNextMatchData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetNextMatchData {
    get_next_match: Option<GetNextMatch>,
}

#[derive(Debug, Deserialize)]
struct GetNextMatch {
    #[serde(rename = "match")]
    match_info: Option<MatchInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchInfo {
    pub id: String,
    pub database_id: u32,
    pub participant1: Participant,
    pub participant2: Participant,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub name: String,
    pub game_display_id: String,
}

const GET_NEXT_MATCH_QUERY: &str = r#"
mutation {
  getNextMatch {
    match {
      id
      databaseId
      participant1 {
        name
        gameDisplayId
      }
      participant2 {
        name
        gameDisplayId
      }
    }
  }
}
"#;

pub async fn get_next_match(website_url: &str, token: &str) -> anyhow::Result<MatchInfo> {
    let client = Client::new();

    let body = serde_json::json!({
        "query": GET_NEXT_MATCH_QUERY,
    });

    let url = format!("{}/graphql/", website_url.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .header("Authorization", format!("Token {}", token))
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to send GraphQL request")?;

    let text = resp.text().await.context("Failed to read response body")?;

    let parsed: GraphQLResponse = serde_json::from_str(&text).context("Failed to parse GraphQL response")?;

    parsed
        .data
        .ok_or_else(|| anyhow!("GraphQL response has no data"))?
        .get_next_match
        .ok_or_else(|| anyhow!("GraphQL response has no getNextMatch"))?
        .match_info
        .ok_or_else(|| anyhow!("GraphQL response has no match"))
}
