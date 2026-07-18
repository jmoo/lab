mod anki;

use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::process;

#[derive(Parser)]
#[command(name = "anki-tool", about = "Query AnkiConnect for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List all deck names
    Decks,
    /// Deck statistics (due counts, totals)
    Stats {
        /// Deck name filter (omit for all decks)
        deck: Option<String>,
    },
    /// Find cards by Anki search query, returns card IDs
    Find {
        /// Anki search query (e.g. "deck:Default", "is:due", "tag:hard")
        query: String,
        /// Also fetch full card info
        #[arg(short, long)]
        info: bool,
        /// Max results to return
        #[arg(short, long, default_value_t = 100)]
        limit: usize,
    },
    /// Get card content and scheduling info
    Cards {
        /// Card IDs
        ids: Vec<i64>,
    },
    /// Find cards due for review
    Due {
        /// Deck name (omit for all)
        deck: Option<String>,
        /// Max results
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },
    /// Find difficult cards (leeches / low ease)
    Hard {
        /// Deck name (omit for all)
        deck: Option<String>,
        /// Max results
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },
    /// Get review history for cards
    Reviews {
        /// Card IDs
        ids: Vec<i64>,
    },
    /// Collection-level stats
    Overview,
    /// List sub-decks with stats for a parent deck
    Subdecks {
        /// Parent deck name prefix
        deck: String,
    },
    /// Study progress summary for a deck tree
    Progress {
        /// Deck name prefix
        deck: String,
    },
    /// Review history by day (from cardReviews API)
    History {
        /// Deck name (omit for all)
        deck: Option<String>,
        /// Number of days to look back
        #[arg(short, long, default_value_t = 56)]
        days: u32,
    },
    /// Today's review session recap
    Session {
        /// Deck name (omit for all)
        deck: Option<String>,
    },
    /// Sync with AnkiWeb
    Sync,
    /// Raw AnkiConnect query (advanced)
    Raw {
        /// Anki search query
        query: String,
        /// Also fetch full card info
        #[arg(short, long)]
        info: bool,
        /// Max results
        #[arg(short, long, default_value_t = 100)]
        limit: usize,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = run(cli.command);
    match result {
        Ok(output) => {
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

fn run(cmd: Cmd) -> Result<serde_json::Value, String> {
    match cmd {
        Cmd::Decks => anki::deck_names(),
        Cmd::Stats { deck } => {
            let decks = resolve_decks(deck)?;
            let stats = anki::deck_stats(&decks)?;
            Ok(serde_json::to_value(stats).unwrap())
        }
        Cmd::Find { query, info, limit } => {
            let ids = anki::find_cards(&query)?;
            let ids: Vec<_> = ids.into_iter().take(limit).collect();
            if !info {
                return Ok(json!({"count": ids.len(), "card_ids": ids}));
            }
            let cards = anki::cards_info(&ids)?;
            let compact: Vec<_> = cards.iter().map(compact_card).collect();
            Ok(json!({"count": compact.len(), "cards": compact}))
        }
        Cmd::Cards { ids } => {
            let cards = anki::cards_info(&ids)?;
            let compact: Vec<_> = cards.iter().map(compact_card).collect();
            Ok(serde_json::to_value(compact).unwrap())
        }
        Cmd::Due { deck, limit } => {
            let query = match &deck {
                Some(d) => format!("is:due deck:{d}"),
                None => "is:due".to_string(),
            };
            let ids = anki::find_cards(&query)?;
            let ids: Vec<_> = ids.into_iter().take(limit).collect();
            if ids.is_empty() {
                return Ok(json!({"count": 0, "cards": []}));
            }
            let cards = anki::cards_info(&ids)?;
            let compact: Vec<_> = cards.iter().map(compact_card).collect();
            Ok(json!({"count": compact.len(), "cards": compact}))
        }
        Cmd::Hard { deck, limit } => {
            let query = match &deck {
                Some(d) => format!("(tag:leech OR prop:ease<1.5) -is:new deck:{d}"),
                None => "(tag:leech OR prop:ease<1.5) -is:new".to_string(),
            };
            let ids = anki::find_cards(&query)?;
            let ids: Vec<_> = ids.into_iter().take(limit).collect();
            if ids.is_empty() {
                return Ok(json!({"count": 0, "cards": []}));
            }
            let cards = anki::cards_info(&ids)?;
            let mut compact: Vec<_> = cards.iter().map(compact_card).collect();
            // Sort by ease ascending (hardest first)
            compact.sort_by(|a, b| {
                let ea = a.get("ease").and_then(|v| v.as_i64()).unwrap_or(9999);
                let eb = b.get("ease").and_then(|v| v.as_i64()).unwrap_or(9999);
                ea.cmp(&eb)
            });
            Ok(json!({"count": compact.len(), "cards": compact}))
        }
        Cmd::Reviews { ids } => {
            let reviews = anki::get_reviews(&ids)?;
            Ok(serde_json::to_value(reviews).unwrap())
        }
        Cmd::Overview => {
            let decks = anki::deck_names()?;
            let deck_list: Vec<String> =
                serde_json::from_value(decks.clone()).map_err(|e| e.to_string())?;
            let stats = anki::deck_stats(&deck_list)?;
            let total_due: u32 = stats.iter().map(|s| s.review_count + s.learn_count).sum();
            let total_new: u32 = stats.iter().map(|s| s.new_count).sum();
            let total_cards: u32 = stats.iter().map(|s| s.total_in_deck).sum();

            // Build recent review history from cardReviews API (last 14 days)
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let start_ms = now_ms - (14 * 86400 * 1000);
            let mut by_day: BTreeMap<String, u32> = BTreeMap::new();
            for deck_name in &deck_list {
                let reviews = anki::card_reviews(deck_name, start_ms)?;
                for rev in reviews {
                    let secs = rev.review_time / 1000;
                    let days_since_epoch = secs / 86400;
                    let date = format_epoch_days(days_since_epoch);
                    *by_day.entry(date).or_default() += 1;
                }
            }
            let reviewed_today = by_day
                .iter()
                .last()
                .filter(|(d, _)| {
                    let today_days = now_ms / 1000 / 86400;
                    **d == format_epoch_days(today_days)
                })
                .map(|(_, c)| *c)
                .unwrap_or(0);
            let recent: Vec<_> = by_day
                .into_iter()
                .map(|(date, count)| json!([date, count]))
                .collect();

            Ok(json!({
                "reviewed_today": reviewed_today,
                "total_due": total_due,
                "total_new": total_new,
                "total_cards": total_cards,
                "deck_count": deck_list.len(),
                "recent_reviews": recent,
            }))
        }
        Cmd::Subdecks { deck } => {
            let all_decks = anki::deck_names()?;
            let all: Vec<String> =
                serde_json::from_value(all_decks).map_err(|e| e.to_string())?;
            let prefix = format!("{deck}::");
            let mut matching: Vec<String> = all
                .into_iter()
                .filter(|d| d.starts_with(&prefix) || d == &deck)
                .collect();
            matching.sort();
            let stats = anki::deck_stats(&matching)?;
            Ok(serde_json::to_value(stats).unwrap())
        }
        Cmd::History { deck, days } => {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let start_ms = now_ms - (days as i64 * 86400 * 1000);

            let all_decks = anki::deck_names()?;
            let all: Vec<String> =
                serde_json::from_value(all_decks).map_err(|e| e.to_string())?;
            let target_decks: Vec<&String> = match &deck {
                Some(d) => {
                    let prefix = format!("{d}::");
                    all.iter()
                        .filter(|name| *name == d || name.starts_with(&prefix))
                        .collect()
                }
                None => all.iter().collect(),
            };

            // Collect all review timestamps
            let mut by_day: BTreeMap<String, u32> = BTreeMap::new();
            for deck_name in &target_decks {
                let reviews = anki::card_reviews(deck_name, start_ms)?;
                for rev in reviews {
                    // Convert ms timestamp to date string
                    let secs = rev.review_time / 1000;
                    let days_since_epoch = secs / 86400;
                    let date = format_epoch_days(days_since_epoch);
                    *by_day.entry(date).or_default() += 1;
                }
            }

            let days: Vec<_> = by_day.into_iter().map(|(date, count)| json!([date, count])).collect();
            Ok(json!({
                "days": days,
                "total": days.iter().map(|d| d[1].as_u64().unwrap_or(0)).sum::<u64>(),
            }))
        }
        Cmd::Session { deck } => {
            // Today's start as ms timestamp (use 4am UTC as Anki's default rollover)
            let today_start = {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                let secs = now / 1000;
                let day_secs = secs - (secs % 86400) + (4 * 3600);
                let day_start = if day_secs > secs { day_secs - 86400 } else { day_secs };
                day_start * 1000
            };

            // Get all decks matching the filter
            let all_decks = anki::deck_names()?;
            let all: Vec<String> =
                serde_json::from_value(all_decks).map_err(|e| e.to_string())?;
            let target_decks: Vec<&String> = match &deck {
                Some(d) => {
                    let prefix = format!("{d}::");
                    all.iter()
                        .filter(|name| *name == d || name.starts_with(&prefix))
                        .collect()
                }
                None => all.iter().collect(),
            };

            // Fetch reviews from cardReviews API for each deck
            let mut card_eases: HashMap<i64, Vec<i32>> = HashMap::new();
            for deck_name in &target_decks {
                let reviews = anki::card_reviews(deck_name, today_start)?;
                for rev in reviews {
                    card_eases
                        .entry(rev.card_id)
                        .or_default()
                        .push(rev.button_pressed);
                }
            }

            if card_eases.is_empty() {
                return Ok(json!({
                    "reviewed": 0,
                    "again": [],
                    "hard": [],
                    "good": [],
                    "easy": [],
                    "victories": [],
                }));
            }

            // Fetch card info for all reviewed cards
            let card_ids: Vec<i64> = card_eases.keys().copied().collect();
            let cards = anki::cards_info(&card_ids)?;
            let card_map: HashMap<i64, &anki::CardInfo> =
                cards.iter().map(|c| (c.card_id, c)).collect();

            let mut again = Vec::new();
            let mut hard = Vec::new();
            let mut good_count = 0u32;
            let mut easy_count = 0u32;
            let mut victories = Vec::new();

            for (&card_id, eases) in &card_eases {
                let Some(card) = card_map.get(&card_id) else { continue };
                let worst_ease = *eases.iter().min().unwrap();
                let final_ease = *eases.last().unwrap();

                match worst_ease {
                    1 => again.push(compact_card(card)),
                    2 => hard.push(compact_card(card)),
                    _ => {
                        if card.lapses >= 3 && final_ease >= 3 {
                            victories.push(compact_card(card));
                        }
                        if worst_ease == 4 {
                            easy_count += 1;
                        } else {
                            good_count += 1;
                        }
                    }
                }
            }

            let sort_by_lapses = |v: &mut Vec<serde_json::Value>| {
                v.sort_by(|a, b| {
                    let la = a.get("lapses").and_then(|v| v.as_i64()).unwrap_or(0);
                    let lb = b.get("lapses").and_then(|v| v.as_i64()).unwrap_or(0);
                    lb.cmp(&la)
                });
            };
            sort_by_lapses(&mut again);
            sort_by_lapses(&mut hard);
            sort_by_lapses(&mut victories);

            Ok(json!({
                "reviewed": card_eases.len(),
                "again": again,
                "hard": hard,
                "good": good_count,
                "easy": easy_count,
                "victories": victories,
            }))
        }
        Cmd::Sync => {
            anki::sync()?;
            Ok(json!({"status": "ok"}))
        }
        Cmd::Progress { deck } => {
            let all_decks = anki::deck_names()?;
            let all: Vec<String> =
                serde_json::from_value(all_decks).map_err(|e| e.to_string())?;
            let prefix = format!("{deck}::");
            let matching: Vec<String> = all
                .into_iter()
                .filter(|d| d.starts_with(&prefix) || d == &deck)
                .collect();
            let stats = anki::deck_stats(&matching)?;

            let total_cards: u32 = stats.iter().map(|s| s.total_in_deck).sum();
            let total_new: u32 = stats.iter().map(|s| s.new_count).sum();
            let total_due: u32 = stats.iter().map(|s| s.review_count + s.learn_count).sum();

            // Count cards by maturity
            let deck_query = format!("\"deck:{deck}\"");
            let mature = anki::find_cards(&format!("{deck_query} prop:ivl>=21"))?.len();
            let young = anki::find_cards(&format!("{deck_query} -is:new -prop:ivl>=21"))?.len();
            let unseen = anki::find_cards(&format!("{deck_query} is:new"))?.len();
            let suspended = anki::find_cards(&format!("{deck_query} is:suspended"))?.len();
            let leeches = anki::find_cards(&format!("{deck_query} tag:leech"))?.len();

            // Per-subdeck breakdown (only direct children)
            let mut subdeck_stats: Vec<serde_json::Value> = Vec::new();
            for s in &stats {
                if s.name == deck || (s.name.starts_with(&prefix) && !s.name[prefix.len()..].contains("::")) {
                    let sq = format!("\"deck:{}\"", s.name);
                    let sm = anki::find_cards(&format!("{sq} prop:ivl>=21"))?.len();
                    let sy = anki::find_cards(&format!("{sq} -is:new -prop:ivl>=21"))?.len();
                    let pct = if s.total_in_deck > 0 {
                        ((sm + sy) as f64 / s.total_in_deck as f64 * 100.0).round() as u32
                    } else {
                        0
                    };
                    subdeck_stats.push(json!({
                        "name": s.name,
                        "total": s.total_in_deck,
                        "mature": sm,
                        "young": sy,
                        "new": s.new_count,
                        "due": s.review_count + s.learn_count,
                        "pct_seen": pct,
                    }));
                }
            }

            let pct_seen = if total_cards > 0 {
                ((mature + young) as f64 / total_cards as f64 * 100.0).round() as u32
            } else {
                0
            };

            Ok(json!({
                "deck": deck,
                "total_cards": total_cards,
                "mature": mature,
                "young": young,
                "unseen": unseen,
                "suspended": suspended,
                "leeches": leeches,
                "due_now": total_due,
                "new_available": total_new,
                "pct_seen": pct_seen,
                "subdecks": subdeck_stats,
            }))
        }
        Cmd::Raw { query, info, limit } => {
            let ids = anki::find_cards(&query)?;
            let ids: Vec<_> = ids.into_iter().take(limit).collect();
            if !info {
                return Ok(json!({"count": ids.len(), "card_ids": ids}));
            }
            let cards = anki::cards_info(&ids)?;
            let compact: Vec<_> = cards.iter().map(compact_card).collect();
            Ok(json!({"count": compact.len(), "cards": compact}))
        }
    }
}

fn resolve_decks(deck: Option<String>) -> Result<Vec<String>, String> {
    match deck {
        Some(d) => Ok(vec![d]),
        None => {
            let names = anki::deck_names()?;
            serde_json::from_value(names).map_err(|e| format!("parse error: {e}"))
        }
    }
}

/// Produce a compact card representation for minimal token output
fn compact_card(card: &anki::CardInfo) -> serde_json::Value {
    let mut fields = BTreeMap::new();
    let mut sorted_fields: Vec<_> = card.fields.iter().collect();
    sorted_fields.sort_by_key(|(_, v)| v.order);
    for (name, fv) in sorted_fields {
        let cleaned = strip_html(&fv.value);
        if !cleaned.is_empty() {
            fields.insert(name.clone(), cleaned);
        }
    }
    json!({
        "id": card.card_id,
        "deck": card.deck_name,
        "model": card.model_name,
        "fields": fields,
        "interval": card.interval,
        "due": card.due,
        "reps": card.reps,
        "lapses": card.lapses,
        "ease": card.factor.unwrap_or(0),
        "type": match card.card_type {
            0 => "new",
            1 => "learning",
            2 => "review",
            3 => "relearning",
            _ => "unknown",
        },
    })
}

/// Convert days since Unix epoch to YYYY-MM-DD string
fn format_epoch_days(days: i64) -> String {
    // Civil date from days since epoch (algorithm from Howard Hinnant)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Minimal HTML stripping - removes tags and decodes common entities
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .trim()
        .to_string()
}
