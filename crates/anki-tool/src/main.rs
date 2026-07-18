mod anki;

use chrono::{Duration, Local, NaiveDate, TimeZone, Timelike};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::process;

/// Scheduler selection for commands whose signals differ by algorithm. `Auto` probes
/// the collection; the explicit values force a scheduler (e.g. for testing).
#[derive(Clone, Copy, ValueEnum)]
enum SchedulerArg {
    Auto,
    Sm2,
    Fsrs,
}

/// Which "struggle" signal `hard` selects on. `Auto` picks the most current-state
/// signal available: retrievability on FSRS, recent-failure on SM-2.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Lens {
    /// Most current-state signal for the active scheduler (default).
    Auto,
    /// About to forget: low FSRS retrievability (`prop:r`). FSRS only.
    Retrievability,
    /// Actively failing: high Again-rate in the recent revlog. Any scheduler.
    Recent,
    /// Intrinsically hard: FSRS difficulty / SM-2 low ease. Any scheduler.
    Difficulty,
}

impl Lens {
    fn name(self) -> &'static str {
        match self {
            Lens::Auto => "auto",
            Lens::Retrievability => "retrievability",
            Lens::Recent => "recent",
            Lens::Difficulty => "difficulty",
        }
    }
}

/// Hour of local time at which Anki rolls over to a new "study day" (Anki's default).
const ROLLOVER_HOUR: i64 = 4;

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
        /// Emit only the match count ({"count": N})
        #[arg(short, long)]
        count: bool,
        /// Group matches by deck ({deck: count}); counts all matches, ignores --limit
        #[arg(short, long)]
        by_deck: bool,
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
        /// Group due cards by deck ({deck: count}) instead of listing cards
        #[arg(short, long)]
        by_deck: bool,
        /// Max results
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },
    /// Per-day forecast of upcoming load (limit-aware when a deck is given)
    Forecast {
        /// Deck name (omit for whole collection; limits only applied with a deck)
        deck: Option<String>,
        /// Number of days to project (day 0 = today)
        #[arg(short, long, default_value_t = 7)]
        days: u32,
    },
    /// New-card queue: unseen pool, daily limit, and how many remain today
    New {
        /// Deck name (omit for all)
        deck: Option<String>,
    },
    /// Struggling cards. Default lens is current-state (retrievability on FSRS)
    Hard {
        /// Deck name (omit for all)
        deck: Option<String>,
        /// Which struggle signal to select on
        #[arg(long, value_enum, default_value_t = Lens::Auto)]
        by: Lens,
        /// Scheduler to assume (auto-detected by default)
        #[arg(long, value_enum, default_value_t = SchedulerArg::Auto)]
        scheduler: SchedulerArg,
        /// retrievability lens: cards with prop:r below this are "about to forget"
        #[arg(long, default_value_t = 0.9)]
        max_retrievability: f64,
        /// difficulty lens (FSRS): cards with prop:d above this count as hard
        #[arg(long, default_value_t = 0.8)]
        min_difficulty: f64,
        /// recent lens: look back this many days in the revlog
        #[arg(long, default_value_t = 14)]
        days: u32,
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
        Cmd::Find {
            query,
            info,
            count,
            by_deck,
            limit,
        } => {
            if by_deck {
                let ids = anki::find_cards(&query)?;
                return group_by_deck(&ids);
            }
            if count {
                return Ok(json!({ "count": anki::count_cards(&query)? }));
            }
            find_output(&query, info, limit)
        }
        Cmd::Cards { ids } => {
            let cards = anki::cards_info(&ids)?;
            let compact: Vec<_> = cards.iter().map(compact_card).collect();
            Ok(serde_json::to_value(compact).unwrap())
        }
        Cmd::Due { deck, by_deck, limit } => {
            let query = scoped_query(&deck, "is:due");
            let ids = anki::find_cards(&query)?;
            if by_deck {
                return group_by_deck(&ids);
            }
            let ids: Vec<_> = ids.into_iter().take(limit).collect();
            cards_output(&ids)
        }
        Cmd::Forecast { deck, days } => forecast(&deck, days),
        Cmd::New { deck } => new_queue(&deck),
        Cmd::Hard {
            deck,
            by,
            scheduler,
            max_retrievability,
            min_difficulty,
            days,
            limit,
        } => {
            let scheduler = resolve_scheduler(scheduler)?;
            let lens = resolve_lens(by, scheduler)?;
            hard(
                scheduler,
                lens,
                &deck,
                max_retrievability,
                min_difficulty,
                days,
                limit,
            )
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

            let reviewed_today = anki::num_reviewed_today()?;

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
                    *by_day.entry(review_date(rev.review_time)).or_default() += 1;
                }
            }
            let recent: Vec<_> = by_day
                .into_iter()
                .map(|(date, count)| json!([date, count]))
                .collect();

            Ok(json!({
                "scheduler": anki::detect_scheduler()?.name(),
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
                    *by_day.entry(review_date(rev.review_time)).or_default() += 1;
                }
            }

            let days: Vec<_> = by_day.into_iter().map(|(date, count)| json!([date, count])).collect();
            Ok(json!({
                "days": days,
                "total": days.iter().map(|d| d[1].as_u64().unwrap_or(0)).sum::<u64>(),
            }))
        }
        Cmd::Session { deck } => {
            let today_start = today_start_ms();

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

                match worst_ease {
                    1 => again.push(compact_card(card)),
                    2 => hard.push(compact_card(card)),
                    _ => {
                        // Reaching this arm means every press today was Good/Easy
                        // (worst_ease >= 3); a card with a long lapse history that
                        // still went clean is a "victory".
                        if card.lapses >= 3 {
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
        Cmd::Raw { query, info, limit } => find_output(&query, info, limit),
    }
}

/// Fetch compact card details for a set of ids, wrapped as `{count, cards}`.
fn cards_output(ids: &[i64]) -> Result<serde_json::Value, String> {
    if ids.is_empty() {
        return Ok(json!({"count": 0, "cards": []}));
    }
    let cards = anki::cards_info(ids)?;
    let compact: Vec<_> = cards.iter().map(compact_card).collect();
    Ok(json!({"count": compact.len(), "cards": compact}))
}

/// Run a search and emit either bare card ids or full card details (with `--info`).
fn find_output(query: &str, info: bool, limit: usize) -> Result<serde_json::Value, String> {
    let ids = anki::find_cards(query)?;
    let ids: Vec<_> = ids.into_iter().take(limit).collect();
    if !info {
        return Ok(json!({"count": ids.len(), "card_ids": ids}));
    }
    cards_output(&ids)
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

/// An Anki search scoped to an optional deck (its whole subtree): `deck:"X" <rest>`,
/// or just `<rest>` for the whole collection. Quoting the deck handles spaces/`::`.
fn scoped_query(deck: &Option<String>, rest: &str) -> String {
    match deck {
        Some(d) => format!("deck:\"{d}\" {rest}"),
        None => rest.to_string(),
    }
}

/// Resolve a CLI scheduler choice, auto-detecting from the collection when asked.
fn resolve_scheduler(arg: SchedulerArg) -> Result<anki::Scheduler, String> {
    match arg {
        SchedulerArg::Auto => anki::detect_scheduler(),
        SchedulerArg::Sm2 => Ok(anki::Scheduler::Sm2),
        SchedulerArg::Fsrs => Ok(anki::Scheduler::Fsrs),
    }
}

/// Resolve the `Auto` lens to the most current-state signal the scheduler supports:
/// retrievability on FSRS, recent-failure on SM-2 (which has no retrievability). An
/// explicit `retrievability` lens on SM-2 is an error.
fn resolve_lens(by: Lens, scheduler: anki::Scheduler) -> Result<Lens, String> {
    match (by, scheduler) {
        (Lens::Auto, anki::Scheduler::Fsrs) => Ok(Lens::Retrievability),
        (Lens::Auto, anki::Scheduler::Sm2) => Ok(Lens::Recent),
        (Lens::Retrievability, anki::Scheduler::Sm2) => {
            Err("retrievability lens requires FSRS; this collection is SM-2 (try --by recent)".into())
        }
        (other, _) => Ok(other),
    }
}

/// Struggling cards under a given lens. Each lens ranks a different question:
/// retrievability = about to forget now, recent = actively failing lately, difficulty
/// = intrinsically hard. Excludes new cards. Output carries the lens + scheduler so the
/// caller knows which signal it's seeing.
fn hard(
    scheduler: anki::Scheduler,
    lens: Lens,
    deck: &Option<String>,
    max_retrievability: f64,
    min_difficulty: f64,
    days: u32,
    limit: usize,
) -> Result<serde_json::Value, String> {
    // Each lens produces card ids in ranked order (worst first) plus per-card
    // annotations to merge into the output, keyed by card id.
    let mut annotations: HashMap<i64, serde_json::Value> = HashMap::new();
    let ranked_ids: Vec<i64> = match lens {
        Lens::Retrievability => retrievability_ranked(deck, max_retrievability)?
            .into_iter()
            .map(|(id, under)| {
                annotations.insert(id, json!({ "r_under": under }));
                id
            })
            .collect(),
        Lens::Recent => recent_struggles(deck, days)?
            .into_iter()
            .map(|(id, again, total)| {
                annotations.insert(id, json!({ "recent_again": again, "recent_reviews": total }));
                id
            })
            .collect(),
        Lens::Difficulty => difficulty_ranked(scheduler, deck, min_difficulty)?,
        Lens::Auto => unreachable!("Auto is resolved before hard()"),
    };

    let ids: Vec<i64> = ranked_ids.into_iter().take(limit).collect();
    if ids.is_empty() {
        return Ok(json!({
            "count": 0, "lens": lens.name(), "scheduler": scheduler.name(), "cards": []
        }));
    }

    // cardsInfo returns cards in arbitrary order; re-emit them in the lens's ranking.
    let cards = anki::cards_info(&ids)?;
    let by_id: HashMap<i64, &anki::CardInfo> = cards.iter().map(|c| (c.card_id, c)).collect();
    let compact: Vec<serde_json::Value> = ids
        .iter()
        .filter_map(|id| by_id.get(id))
        .map(|c| {
            let mut v = compact_card(c);
            if let Some(extra) = annotations.get(&c.card_id).and_then(|a| a.as_object()) {
                for (k, val) in extra {
                    v[k.clone()] = val.clone();
                }
            }
            v
        })
        .collect();

    Ok(json!({
        "count": compact.len(),
        "lens": lens.name(),
        "scheduler": scheduler.name(),
        "cards": compact,
    }))
}

/// Intrinsically-hard cards, ranked. SM-2 keys on low ease (leech or ease<1.5), FSRS on
/// high difficulty (leech or prop:d over the threshold). Ranked worst-first: by ease
/// ascending on SM-2, by lapses descending on FSRS (ease is stale there).
fn difficulty_ranked(
    scheduler: anki::Scheduler,
    deck: &Option<String>,
    min_difficulty: f64,
) -> Result<Vec<i64>, String> {
    let signal = match scheduler {
        anki::Scheduler::Sm2 => "tag:leech OR prop:ease<1.5".to_string(),
        anki::Scheduler::Fsrs => format!("tag:leech OR prop:d>{min_difficulty}"),
    };
    let ids = anki::find_cards(&scoped_query(deck, &format!("({signal}) -is:new")))?;
    if ids.is_empty() {
        return Ok(ids);
    }
    let cards = anki::cards_info(&ids)?;
    let mut cards: Vec<&anki::CardInfo> = cards.iter().collect();
    match scheduler {
        anki::Scheduler::Sm2 => {
            cards.sort_by_key(|c| c.factor.unwrap_or(i32::MAX));
        }
        anki::Scheduler::Fsrs => {
            cards.sort_by_key(|c| std::cmp::Reverse(c.lapses));
        }
    }
    Ok(cards.into_iter().map(|c| c.card_id).collect())
}

/// Cards below the retrievability threshold, ranked most-at-risk first. Since
/// `cardsInfo` doesn't return per-card `r`, we approximate each card's r by the
/// smallest threshold bucket it falls into (a lower bucket = lower r = more at risk).
/// Returns `(card_id, r_under)` where `r_under` is that bucket's upper bound.
fn retrievability_ranked(
    deck: &Option<String>,
    max_retrievability: f64,
) -> Result<Vec<(i64, f64)>, String> {
    // Ascending edges up to (and including) the requested max. Smallest first so the
    // first bucket a card appears in is its tightest upper bound on r.
    let mut edges: Vec<f64> = [0.5, 0.6, 0.7, 0.8, 0.9]
        .into_iter()
        .filter(|&e| e < max_retrievability)
        .collect();
    edges.push(max_retrievability);

    let mut bucket: HashMap<i64, f64> = HashMap::new();
    for &e in &edges {
        let q = scoped_query(deck, &format!("prop:r<{e} -is:new"));
        for id in anki::find_cards(&q)? {
            bucket.entry(id).or_insert(e);
        }
    }
    let mut v: Vec<(i64, f64)> = bucket.into_iter().collect();
    v.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(v)
}

/// Cards with at least one Again (button 1) in the last `days`, ranked by Again-rate
/// (then Again-count). Mines the deck revlog via `cardReviews`. Returns
/// `(card_id, again_count, total_reviews)`.
fn recent_struggles(deck: &Option<String>, days: u32) -> Result<Vec<(i64, u32, u32)>, String> {
    let start_ms = today_start_ms() - (days as i64 * 86400 * 1000);
    let mut agg: HashMap<i64, (u32, u32)> = HashMap::new();
    for name in target_decks(deck)? {
        for rev in anki::card_reviews(&name, start_ms)? {
            let e = agg.entry(rev.card_id).or_default();
            e.1 += 1;
            if rev.button_pressed == 1 {
                e.0 += 1;
            }
        }
    }
    let mut v: Vec<(i64, u32, u32)> = agg
        .into_iter()
        .filter(|(_, (again, _))| *again > 0)
        .map(|(id, (a, t))| (id, a, t))
        .collect();
    v.sort_by(|&(_, aa, ta), &(_, ab, tb)| rank_recent(aa, ta, ab, tb));
    Ok(v)
}

/// Ordering for the recent lens: higher Again-rate first, ties broken by Again-count.
fn rank_recent(aa: u32, ta: u32, ab: u32, tb: u32) -> std::cmp::Ordering {
    let ra = aa as f64 / ta as f64;
    let rb = ab as f64 / tb as f64;
    rb.partial_cmp(&ra)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(ab.cmp(&aa))
}

/// Decks matching an optional filter: the deck itself plus its subdecks, or all.
fn target_decks(deck: &Option<String>) -> Result<Vec<String>, String> {
    let all: Vec<String> =
        serde_json::from_value(anki::deck_names()?).map_err(|e| e.to_string())?;
    Ok(match deck {
        Some(d) => {
            let prefix = format!("{d}::");
            all.into_iter()
                .filter(|name| name == d || name.starts_with(&prefix))
                .collect()
        }
        None => all,
    })
}

/// Group a set of card ids by their deck, as `{count, by_deck: {deck: n}}`.
fn group_by_deck(ids: &[i64]) -> Result<serde_json::Value, String> {
    if ids.is_empty() {
        return Ok(json!({"count": 0, "by_deck": {}}));
    }
    let cards = anki::cards_info(ids)?;
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for c in &cards {
        *counts.entry(c.deck_name.clone()).or_default() += 1;
    }
    Ok(json!({"count": cards.len(), "by_deck": counts}))
}

/// The current study day as a local date (honors the rollover hour).
fn study_today() -> NaiveDate {
    Local
        .timestamp_millis_opt(today_start_ms())
        .single()
        .expect("valid day start")
        .date_naive()
}

/// study-today + `offset` days, as `YYYY-MM-DD`.
fn date_offset(offset: i64) -> String {
    (study_today() + Duration::days(offset))
        .format("%Y-%m-%d")
        .to_string()
}

/// Per-day forecast of upcoming review load, projected purely from the CURRENT state
/// of the collection: for each day it counts the review/learning cards already
/// scheduled to come due (Anki's "Future Due" graph). Day 0 is everything due now
/// (including overdue), later days are cards due exactly that many days out.
///
/// It deliberately assumes nothing about future study behavior — new cards are not
/// distributed across days (they only enter the queue if you study them) and nothing
/// is clamped to daily limits (that would presume you review exactly the cap). The
/// unseen new pool and the deck's configured limits are reported as context, not
/// projected. A behavioral study simulation would be a separate, opt-in mode.
fn forecast(deck: &Option<String>, days: u32) -> Result<serde_json::Value, String> {
    let mut rows = Vec::with_capacity(days as usize);
    let mut total_due = 0i64;
    for d in 0..days as i64 {
        let q = if d == 0 {
            "is:due".to_string()
        } else {
            format!("prop:due={d}")
        };
        let due = anki::count_cards(&scoped_query(deck, &q))? as i64;
        total_due += due;
        rows.push(json!({"day": d, "date": date_offset(d), "due": due}));
    }

    let unseen = anki::count_cards(&scoped_query(deck, "is:new -is:suspended"))?;
    let limits = match deck {
        Some(d) => Some(anki::deck_limits(d)?),
        None => None,
    };

    Ok(json!({
        "deck": deck,
        "days": days,
        "forecast": rows,
        "total_due": total_due,
        "unseen": unseen,
        "new_per_day": limits.as_ref().map(|l| l.new_per_day),
        "rev_per_day": limits.as_ref().map(|l| l.rev_per_day),
    }))
}

/// New-card queue snapshot: unseen pool, plus (with a deck) the per-day new limit
/// and how many may still be introduced today.
fn new_queue(deck: &Option<String>) -> Result<serde_json::Value, String> {
    let unseen = anki::count_cards(&scoped_query(deck, "is:new -is:suspended"))?;
    let introduced_today = new_introduced_today(deck)?;

    let mut out = json!({
        "deck": deck,
        "unseen": unseen,
        "introduced_today": introduced_today,
    });
    if let Some(d) = deck {
        let limits = anki::deck_limits(d)?;
        let remaining = (limits.new_per_day as i64 - introduced_today as i64).max(0);
        out["new_per_day"] = json!(limits.new_per_day);
        out["remaining_today"] = json!(remaining);
    }
    Ok(out)
}

/// Distinct cards whose first study (revlog type 0 = learn) happened today —
/// i.e. new cards introduced in the current study day.
fn new_introduced_today(deck: &Option<String>) -> Result<usize, String> {
    let today_start = today_start_ms();
    let mut introduced: HashSet<i64> = HashSet::new();
    for name in target_decks(deck)? {
        for rev in anki::card_reviews(&name, today_start)? {
            if rev.review_type == 0 {
                introduced.insert(rev.card_id);
            }
        }
    }
    Ok(introduced.len())
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

/// The local study-day (YYYY-MM-DD) a review at `ms` belongs to, honoring Anki's
/// rollover hour — a review before rollover counts toward the previous day.
fn review_date(ms: i64) -> String {
    let dt = Local
        .timestamp_millis_opt(ms - ROLLOVER_HOUR * 3_600_000)
        .single()
        .expect("valid review timestamp");
    dt.format("%Y-%m-%d").to_string()
}

/// Milliseconds at the start of the current local study day (the most recent rollover).
fn today_start_ms() -> i64 {
    let now = Local::now();
    let rollover_secs = ROLLOVER_HOUR * 3600;
    let local_midnight = now.timestamp() - now.num_seconds_from_midnight() as i64;
    let mut boundary = local_midnight + rollover_secs;
    if now.timestamp() < boundary {
        boundary -= 86400;
    }
    boundary * 1000
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
    decode_entities(&out).trim().to_string()
}

/// Decode the HTML entities that show up in Anki fields: the common named ones
/// plus numeric (`&#12354;`) and hex (`&#x3042;`) character references.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp..];
        let decoded = after.find(';').and_then(|semi| {
            let entity = &after[1..semi];
            let c = match entity {
                "nbsp" => Some(' '),
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse::<u32>().ok()))
                    .and_then(char::from_u32),
            };
            c.map(|c| (c, semi))
        });
        match decoded {
            Some((c, semi)) => {
                out.push(c);
                rest = &after[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_named_entities() {
        assert_eq!(strip_html("<b>hi</b>&nbsp;there"), "hi there");
        assert_eq!(strip_html("a &amp; b &lt;c&gt;"), "a & b <c>");
    }

    #[test]
    fn decodes_numeric_and_hex_entities() {
        assert_eq!(strip_html("&#12354;&#x3042;"), "ああ");
        // An unterminated or unknown entity is left untouched.
        assert_eq!(strip_html("100% &amp cats & dogs"), "100% &amp cats & dogs");
    }

    #[test]
    fn review_before_rollover_counts_as_previous_day() {
        // A review one ms before the day's start belongs to the previous study day.
        let start = today_start_ms();
        assert_ne!(review_date(start), review_date(start - 1));
    }

    #[test]
    fn auto_lens_is_current_state_per_scheduler() {
        // FSRS → retrievability (native current state); SM-2 → recent (no r there).
        assert!(matches!(
            resolve_lens(Lens::Auto, anki::Scheduler::Fsrs),
            Ok(Lens::Retrievability)
        ));
        assert!(matches!(
            resolve_lens(Lens::Auto, anki::Scheduler::Sm2),
            Ok(Lens::Recent)
        ));
        // Explicit retrievability on SM-2 is an error; explicit lenses pass through.
        assert!(resolve_lens(Lens::Retrievability, anki::Scheduler::Sm2).is_err());
        assert!(matches!(
            resolve_lens(Lens::Difficulty, anki::Scheduler::Fsrs),
            Ok(Lens::Difficulty)
        ));
    }

    #[test]
    fn recent_lens_ranks_by_again_rate_then_count() {
        use std::cmp::Ordering;
        // 3/4 (0.75) is worse than 1/2 (0.5).
        assert_eq!(rank_recent(3, 4, 1, 2), Ordering::Less);
        // Equal rate (2/4 vs 1/2 = 0.5): more absolute Agains ranks first.
        assert_eq!(rank_recent(2, 4, 1, 2), Ordering::Less);
    }
}
