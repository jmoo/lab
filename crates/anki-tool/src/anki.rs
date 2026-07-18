use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

const ANKI_URL: &str = "http://localhost:8765";

#[derive(Deserialize)]
struct AnkiResponse {
    result: Value,
    error: Option<String>,
}

fn request(action: &str, params: Option<Value>) -> Result<Value, String> {
    let mut body = json!({"action": action, "version": 6});
    if let Some(p) = params {
        body["params"] = p;
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(10))
        .build();
    let resp: AnkiResponse = agent
        .post(ANKI_URL)
        .send_json(&body)
        .map_err(|e| format!("AnkiConnect request failed: {e}"))?
        .into_json()
        .map_err(|e| format!("invalid response: {e}"))?;
    if let Some(err) = resp.error {
        return Err(err);
    }
    Ok(resp.result)
}

// --- Public query functions ---

pub fn deck_names() -> Result<Value, String> {
    request("deckNames", None)
}

/// Full deck name -> deck id. Used to restore full names on getDeckStats output.
pub fn deck_names_and_ids() -> Result<HashMap<String, u64>, String> {
    let result = request("deckNamesAndIds", None)?;
    serde_json::from_value(result).map_err(|e| format!("parse error: {e}"))
}

#[derive(Serialize, Deserialize)]
pub struct DeckStats {
    pub deck_id: u64,
    pub name: String,
    pub new_count: u32,
    pub learn_count: u32,
    pub review_count: u32,
    pub total_in_deck: u32,
}

pub fn deck_stats(decks: &[String]) -> Result<Vec<DeckStats>, String> {
    let result = request("getDeckStats", Some(json!({"decks": decks})))?;
    let map: HashMap<String, DeckStats> =
        serde_json::from_value(result).map_err(|e| format!("parse error: {e}"))?;
    // getDeckStats returns short (leaf) names; restore the full names by deck id,
    // which is unambiguous even when two decks share a leaf name.
    let id_to_full: HashMap<u64, String> = deck_names_and_ids()?
        .into_iter()
        .map(|(name, id)| (id, name))
        .collect();
    let mut stats: Vec<DeckStats> = map
        .into_values()
        .map(|mut s| {
            if let Some(full) = id_to_full.get(&s.deck_id) {
                s.name = full.clone();
            }
            s
        })
        .collect();
    stats.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(stats)
}

pub fn find_cards(query: &str) -> Result<Vec<i64>, String> {
    let result = request("findCards", Some(json!({"query": query})))?;
    serde_json::from_value(result).map_err(|e| format!("parse error: {e}"))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardInfo {
    pub card_id: i64,
    pub deck_name: String,
    pub model_name: String,
    pub fields: HashMap<String, FieldValue>,
    pub interval: i32,
    pub note: i64,
    #[serde(rename = "type")]
    pub card_type: i32,
    pub queue: i32,
    pub due: i64,
    pub reps: i32,
    pub lapses: i32,
    pub factor: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct FieldValue {
    pub value: String,
    pub order: i32,
}

pub fn cards_info(card_ids: &[i64]) -> Result<Vec<CardInfo>, String> {
    let result = request("cardsInfo", Some(json!({"cards": card_ids})))?;
    serde_json::from_value(result).map_err(|e| format!("parse error: {e}"))
}

#[derive(Serialize, Deserialize)]
pub struct ReviewEntry {
    pub id: i64,
    pub ease: i32,
    pub ivl: i64,
    #[serde(rename = "lastIvl")]
    pub last_ivl: i64,
    pub factor: i32,
    pub time: i64,
    #[serde(rename = "type")]
    pub review_type: i32,
}

pub fn get_reviews(card_ids: &[i64]) -> Result<HashMap<String, Vec<ReviewEntry>>, String> {
    let str_ids: Vec<String> = card_ids.iter().map(|id| id.to_string()).collect();
    let result = request("getReviewsOfCards", Some(json!({"cards": str_ids})))?;
    serde_json::from_value(result).map_err(|e| format!("parse error: {e}"))
}

/// Get all reviews for a deck since a given review ID (ms timestamp).
/// cardReviews returns arrays of 9 elements.
pub struct DeckReviewEntry {
    pub review_time: i64,
    pub card_id: i64,
    pub button_pressed: i32,
}

pub fn card_reviews(deck: &str, start_id: i64) -> Result<Vec<DeckReviewEntry>, String> {
    let result = request(
        "cardReviews",
        Some(json!({"deck": deck, "startID": start_id})),
    )?;
    // Response is an array of 9-element arrays
    let rows: Vec<Vec<Value>> =
        serde_json::from_value(result).map_err(|e| format!("parse error: {e}"))?;
    let mut entries = Vec::new();
    for row in rows {
        if row.len() >= 9 {
            // Format: [reviewTime, cardID, usn, buttonPressed, newIvl, prevIvl, newFactor, reviewDuration, reviewType]
            let review_time = row[0].as_i64().unwrap_or(0);
            let card_id = row[1].as_i64().unwrap_or(0);
            let button = row[3].as_i64().unwrap_or(0) as i32;
            entries.push(DeckReviewEntry {
                review_time,
                card_id,
                button_pressed: button,
            });
        }
    }
    Ok(entries)
}

/// Cards reviewed today, using Anki's own local day rollover.
pub fn num_reviewed_today() -> Result<u32, String> {
    let result = request("getNumCardsReviewedToday", None)?;
    serde_json::from_value(result).map_err(|e| format!("parse error: {e}"))
}

pub fn sync() -> Result<Value, String> {
    request("sync", None)
}
