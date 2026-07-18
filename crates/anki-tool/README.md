# anki-tool

CLI for querying AnkiConnect, designed for AI agent consumption. All output is compact JSON to minimize token usage.

## Prerequisites

- [Anki](https://apps.ankiweb.net/) running with [AnkiConnect](https://ankiweb.net/shared/info/2055492159) addon installed (listens on `localhost:8765`)
- [Nix](https://nixos.org/) with flakes enabled

## Install / Run

```bash
# Run directly
nix run github:jmoo/anki-tool -- <command>

# Or from a local checkout
nix run .# -- <command>
```

## Commands

### `decks` — List all deck names

```bash
nix run .# -- decks
# ["Default","Japanese::Vocab","Japanese::Grammar"]
```

### `stats [DECK]` — Deck statistics

Shows new/learning/review counts and total cards. Omit deck name for all decks.

```bash
nix run .# -- stats
nix run .# -- stats "Japanese::Vocab"
# [{"name":"Japanese::Vocab","new_count":20,"learn_count":3,"review_count":45,"total_in_deck":1200}]
```

### `due [DECK]` — Cards due for review

Returns card content and scheduling info for due cards.

```bash
nix run .# -- due
nix run .# -- due "Japanese::Vocab" --limit 10
# {"count":10,"cards":[{"id":123,"deck":"Japanese::Vocab","fields":{"Front":"食べる","Back":"to eat"},"interval":7,"ease":2500,...}]}
```

### `hard [DECK]` — Difficult cards (leeches / low ease)

Finds cards tagged as leeches or with ease factor below 1.5. Sorted hardest first.

```bash
nix run .# -- hard
nix run .# -- hard "Japanese::Vocab" --limit 20
# {"count":5,"cards":[{"id":456,"fields":{"Front":"難しい","Back":"difficult"},"ease":1300,...}]}
```

### `find <QUERY>` — Search cards

Uses [Anki's search syntax](https://docs.ankiweb.net/searching.html). Returns card IDs by default, or full card details with `--info`.

```bash
nix run .# -- find "deck:Japanese tag:verb"
nix run .# -- find "is:new" --limit 50
nix run .# -- find "added:7"           # cards added in last 7 days
nix run .# -- find "rated:1"           # cards reviewed today
nix run .# -- find "prop:ivl>30"       # cards with interval > 30 days
# {"count":25,"card_ids":[123,456,789,...]}

# With --info, returns full card details instead of just IDs
nix run .# -- find "それから deck:\"日本語::2 - Genki 1::L04\"" --info
# {"count":2,"cards":[{"id":...,"deck":"...","fields":{"english":"and then","japanese_kana":"それから"},...}]}
```

### `cards <ID...>` — Get card details

Fetch full content and scheduling info for specific card IDs.

```bash
nix run .# -- cards 1494723142483 1494703460437
# [{"id":1494723142483,"deck":"Default","model":"Basic","fields":{"Front":"hello","Back":"world"},"interval":16,"due":1,"reps":5,"lapses":0,"ease":2500,"type":"review"}]
```

### `reviews <ID...>` — Review history

Get the full review log for specific cards. Each entry shows: ease button pressed (1-4), new interval, time spent (ms), and review type (0=learn, 1=review, 2=relearn, 3=filtered).

```bash
nix run .# -- reviews 1494723142483
# {"1494723142483":[{"id":1653772912146,"ease":3,"ivl":7,"last_ivl":3,"factor":2500,"time":8500,"review_type":1}]}
```

### `overview` — Collection-level stats

Quick summary: cards reviewed today, total due, total new, total cards, and last 14 days of review counts.

```bash
nix run .# -- overview
# {"reviewed_today":42,"total_due":156,"total_new":300,"total_cards":5000,"deck_count":8,"recent_reviews":[["2025-01-01",35],...]}
```

### `subdecks <DECK>` — List sub-decks with stats

Shows all sub-decks under a parent deck with their due counts and totals.

```bash
nix run .# -- subdecks "日本語"
# [{"deck_id":...,"name":"日本語::2 - Genki 1","new_count":20,"learn_count":0,"review_count":78,"total_in_deck":0},...]
```

### `progress <DECK>` — Study progress summary

Comprehensive progress breakdown: mature/young/unseen counts, leeches, and per-subdeck stats with percent-seen.

```bash
nix run .# -- progress "日本語"
# {"deck":"日本語","total_cards":3541,"mature":745,"young":322,"unseen":2474,"pct_seen":30,"due_now":328,"leeches":0,"subdecks":[...]}
```

### `session [DECK]` — Today's review session recap

Shows what happened in today's reviews: which cards you lapsed on (Again), found hard, got right (Good/Easy), and "victories" — cards with 3+ all-time lapses that you answered correctly today.

```bash
nix run .# -- session "日本語"
# {"reviewed":103,"again":[...],"hard":[...],"good":78,"easy":6,"victories":[...]}
```

- `again` / `hard` — full card details, sorted by all-time lapses (worst offenders first)
- `good` / `easy` — counts only (no card details, keeps output compact)
- `victories` — cards with 3+ all-time lapses that were answered correctly this session

### `sync` — Sync with AnkiWeb

Triggers a sync with AnkiWeb. Run this if reviews or cards seem missing or stale, or if the user asks you to sync.

```bash
nix run .# -- sync
# {"status":"ok"}
```

> **Note for AI agents:** If review counts or card data look unexpectedly empty or outdated, run `sync` first and retry your query. The local Anki database may be out of date with AnkiWeb.

### `raw <QUERY>` — Raw search with optional card info

For advanced queries. Add `--info` to also fetch full card details.

```bash
nix run .# -- raw "prop:ease<1.3 deck:Japanese" --info --limit 20
```

## Common AI Agent Workflows

**Daily study overview:**
```bash
nix run .# -- overview
```

**Review tough cards in a deck:**
```bash
nix run .# -- hard "Japanese::Vocab" --limit 20
```

**Find cards on a topic:**
```bash
nix run .# -- find "deck:Biology mitochondria" --limit 10
# then fetch details:
nix run .# -- cards <ids from above>
```

**Analyze struggle patterns:**
```bash
nix run .# -- hard "Japanese" --limit 10
# then get review history for those cards:
nix run .# -- reviews <card_ids>
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
| `prop:ease<N` | Ease factor less than N (e.g. 1.5) |
| `prop:lapses>N` | More than N lapses |
| `"exact phrase"` | Search for exact text |
| `-tag:name` | Exclude tag |

## Design Philosophy

- If a query is too complex for existing commands, add a new command to anki-tool rather than scripting with python/awk/etc.
- All processing logic should live in the tool itself so AI agents can use it without extra dependencies.
