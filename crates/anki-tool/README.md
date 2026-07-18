# anki-tool

CLI for querying AnkiConnect, designed for AI agent consumption. All output is compact JSON to minimize token usage.

## Prerequisites

- [Anki](https://apps.ankiweb.net/) running with [AnkiConnect](https://ankiweb.net/shared/info/2055492159) addon installed (listens on `localhost:8765`)
- [Nix](https://nixos.org/) with flakes enabled

## Install / Run

`anki-tool` lives in the [`jmoo/lab`](https://github.com/jmoo/lab) flake:

```bash
nix run .#anki-tool -- <command>
```

## Commands

### `decks` — List all deck names

```bash
nix run .#anki-tool -- decks
# ["Default","Japanese::Vocab","Japanese::Grammar"]
```

### `stats [DECK]` — Deck statistics

Shows new/learning/review counts and total cards. Omit deck name for all decks.

```bash
nix run .#anki-tool -- stats
nix run .#anki-tool -- stats "Japanese::Vocab"
# [{"name":"Japanese::Vocab","new_count":20,"learn_count":3,"review_count":45,"total_in_deck":1200}]
```

### `due [DECK]` — Cards due for review

Returns card content and scheduling info for due cards.

```bash
nix run .#anki-tool -- due
nix run .#anki-tool -- due "Japanese::Vocab" --limit 10
# {"count":10,"cards":[{"id":123,"deck":"Japanese::Vocab","fields":{"Front":"食べる","Back":"to eat"},"interval":7,"ease":2500,...}]}

# --by-deck groups the due cards by deck instead of listing them
nix run .#anki-tool -- due "日本語" --by-deck
# {"count":98,"by_deck":{"日本語::2 - Genki 1::L06::01 Vocabulary":23,"日本語::2 - Genki 1::L07::01 Vocabulary":26,...}}
```

### `forecast [DECK]` — Upcoming daily load

Projects the next N days of study load (day 0 = today). **When a deck is given** it
reads that deck's option-group limits (`getDeckConfig`) and simulates Anki's queue:
reviews are clamped to the daily review cap with the overflow rolling forward as
`backlog`, and new cards are introduced up to the daily new cap until the unseen pool
empties. Day 0's new allowance is reduced by cards already introduced today, so it
reflects what's actually left for the rest of today. **Without a deck**, limits can't
be attributed, so review counts are reported raw (like Anki's Future Due) and `new`
is 0.

> Approximation: new cards introduced today graduate into reviews on later days; that
> feedback isn't simulated, so far-out review counts are slight underestimates.

```bash
nix run .#anki-tool -- forecast "日本語::2 - Genki 1" --days 7
# {"deck":"日本語::2 - Genki 1","days":7,"limits_applied":true,"new_per_day":20,"rev_per_day":120,
#  "unseen_pool":985,"new_pool_remaining_after":845,"totals":{"new":140,"reviews":280,"cards":420},
#  "forecast":[{"day":0,"date":"2026-07-18","new":20,"reviews":98,"total":118,"backlog":0},...]}

nix run .#anki-tool -- forecast --days 4        # whole collection, raw (limits_applied:false)
```

### `new [DECK]` — New-card queue

Unseen (new, non-suspended) card pool, how many new cards were already introduced
today, and — when a deck is given — the deck's daily new limit and how many may still
be introduced today.

```bash
nix run .#anki-tool -- new "日本語::2 - Genki 1"
# {"deck":"日本語::2 - Genki 1","unseen":985,"introduced_today":0,"new_per_day":20,"remaining_today":20}
nix run .#anki-tool -- new                       # collection-wide: {"unseen":...,"introduced_today":...}
```

### `hard [DECK]` — Difficult cards (leeches / low ease)

Finds cards tagged as leeches or with ease factor below 1.5. Sorted hardest first.

```bash
nix run .#anki-tool -- hard
nix run .#anki-tool -- hard "Japanese::Vocab" --limit 20
# {"count":5,"cards":[{"id":456,"fields":{"Front":"難しい","Back":"difficult"},"ease":1300,...}]}
```

### `find <QUERY>` — Search cards

Uses [Anki's search syntax](https://docs.ankiweb.net/searching.html). Returns card IDs by default, or full card details with `--info`.

```bash
nix run .#anki-tool -- find "deck:Japanese tag:verb"
nix run .#anki-tool -- find "is:new" --limit 50
nix run .#anki-tool -- find "added:7"           # cards added in last 7 days
nix run .#anki-tool -- find "rated:1"           # cards reviewed today
nix run .#anki-tool -- find "prop:ivl>30"       # cards with interval > 30 days
# {"count":25,"card_ids":[123,456,789,...]}

# --count returns only the match count (no ID array)
nix run .#anki-tool -- find "deck:日本語 is:due" --count
# {"count":98}

# --by-deck groups all matches by deck (ignores --limit) — e.g. tomorrow's due, by lesson
nix run .#anki-tool -- find "deck:日本語 prop:due=1" --by-deck
# {"count":52,"by_deck":{"日本語::2 - Genki 1::L06::01 Vocabulary":9,"日本語::2 - Genki 1::L07::01 Vocabulary":15,...}}

# With --info, returns full card details instead of just IDs
nix run .#anki-tool -- find "それから deck:\"日本語::2 - Genki 1::L04\"" --info
# {"count":2,"cards":[{"id":...,"deck":"...","fields":{"english":"and then","japanese_kana":"それから"},...}]}
```

### `cards <ID...>` — Get card details

Fetch full content and scheduling info for specific card IDs.

```bash
nix run .#anki-tool -- cards 1494723142483 1494703460437
# [{"id":1494723142483,"deck":"Default","model":"Basic","fields":{"Front":"hello","Back":"world"},"interval":16,"due":1,"reps":5,"lapses":0,"ease":2500,"type":"review"}]
```

### `reviews <ID...>` — Review history

Get the full review log for specific cards. Each entry shows: ease button pressed (1-4), new interval, time spent (ms), and review type (0=learn, 1=review, 2=relearn, 3=filtered).

```bash
nix run .#anki-tool -- reviews 1494723142483
# {"1494723142483":[{"id":1653772912146,"ease":3,"ivl":7,"last_ivl":3,"factor":2500,"time":8500,"review_type":1}]}
```

### `overview` — Collection-level stats

Quick summary: cards reviewed today, total due, total new, total cards, and last 14 days of review counts.

```bash
nix run .#anki-tool -- overview
# {"reviewed_today":42,"total_due":156,"total_new":300,"total_cards":5000,"deck_count":8,"recent_reviews":[["2025-01-01",35],...]}
```

### `subdecks <DECK>` — List sub-decks with stats

Shows all sub-decks under a parent deck with their due counts and totals.

```bash
nix run .#anki-tool -- subdecks "日本語"
# [{"deck_id":...,"name":"日本語::2 - Genki 1","new_count":20,"learn_count":0,"review_count":78,"total_in_deck":0},...]
```

### `progress <DECK>` — Study progress summary

Comprehensive progress breakdown: mature/young/unseen counts, leeches, and per-subdeck stats with percent-seen.

```bash
nix run .#anki-tool -- progress "日本語"
# {"deck":"日本語","total_cards":3541,"mature":745,"young":322,"unseen":2474,"pct_seen":30,"due_now":328,"leeches":0,"subdecks":[...]}
```

### `session [DECK]` — Today's review session recap

Shows what happened in today's reviews: which cards you lapsed on (Again), found hard, got right (Good/Easy), and "victories" — cards with 3+ all-time lapses that you answered correctly today.

```bash
nix run .#anki-tool -- session "日本語"
# {"reviewed":103,"again":[...],"hard":[...],"good":78,"easy":6,"victories":[...]}
```

- `again` / `hard` — full card details, sorted by all-time lapses (worst offenders first)
- `good` / `easy` — counts only (no card details, keeps output compact)
- `victories` — cards with 3+ all-time lapses that were answered correctly this session

### `sync` — Sync with AnkiWeb

Triggers a sync with AnkiWeb. Run this if reviews or cards seem missing or stale, or if the user asks you to sync.

```bash
nix run .#anki-tool -- sync
# {"status":"ok"}
```

> **Note for AI agents:** If review counts or card data look unexpectedly empty or outdated, run `sync` first and retry your query. The local Anki database may be out of date with AnkiWeb.

### `raw <QUERY>` — Raw search with optional card info

For advanced queries. Add `--info` to also fetch full card details.

```bash
nix run .#anki-tool -- raw "prop:ease<1.3 deck:Japanese" --info --limit 20
```

## Common AI Agent Workflows

**Daily study overview:**
```bash
nix run .#anki-tool -- overview
```

**Review tough cards in a deck:**
```bash
nix run .#anki-tool -- hard "Japanese::Vocab" --limit 20
```

**Find cards on a topic:**
```bash
nix run .#anki-tool -- find "deck:Biology mitochondria" --limit 10
# then fetch details:
nix run .#anki-tool -- cards <ids from above>
```

**Analyze struggle patterns:**
```bash
nix run .#anki-tool -- hard "Japanese" --limit 10
# then get review history for those cards:
nix run .#anki-tool -- reviews <card_ids>
```

## Output Format

All commands output a single line of JSON. Card objects have this shape:

```json
{
  "id": 1494723142483,
  "deck": "Japanese::Vocab",
  "model": "Basic",
  "fields": {"Front": "食べる", "Back": "to eat"},
  "interval": 16,
  "due": 1,
  "reps": 5,
  "lapses": 0,
  "ease": 2500,
  "type": "review"
}
```

- `ease`: 2500 = 250% (Anki's default). Lower = harder.
- `interval`: days until next review
- `lapses`: number of times the card was forgotten
- `type`: one of `new`, `learning`, `review`, `relearning`
- `fields`: HTML-stripped card content, keyed by field name

## Anki Search Syntax Quick Reference

| Query | Meaning |
|-------|---------|
| `deck:Name` | Cards in deck |
| `tag:name` | Cards with tag |
| `is:due` | Due for review |
| `is:new` | New cards |
| `is:learn` | Currently learning |
| `is:review` | Review cards |
| `is:suspended` | Suspended cards |
| `tag:leech` | Leech cards |
| `added:N` | Added in last N days |
| `rated:N` | Reviewed in last N days |
| `prop:ivl>N` | Interval greater than N days |
| `prop:due=N` | Due exactly N days from today (0=today, 1=tomorrow) |
| `prop:ease<N` | Ease factor less than N (e.g. 1.5) |
| `prop:lapses>N` | More than N lapses |
| `"exact phrase"` | Search for exact text |
| `-tag:name` | Exclude tag |

## Design Philosophy

- If a query is too complex for existing commands, add a new command to anki-tool rather than scripting with python/awk/etc.
- All processing logic should live in the tool itself so AI agents can use it without extra dependencies.
