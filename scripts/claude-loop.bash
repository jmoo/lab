#!/usr/bin/env bash
# nix-deps: jq coreutils gnugrep

# claude-loop <session-name|session-id>
#
# Resumes a Claude Code session headlessly and drives it to completion, waiting
# out usage limits along the way. Each iteration attempts to resume the session
# (this doubles as the "is usage available?" probe): if Claude is rate/usage
# limited the attempt returns immediately with a limit error, so we sleep until
# the reset time and probe again. Once tokens are available the resume runs the
# turn to completion, and the script prints the result and exits 0.
#
# `claude` itself is expected on PATH (the user's own Claude Code install, which
# owns ~/.claude) — deliberately not pinned as a nix dependency.
#
# Env overrides:
#   CLAUDE_LOOP_PROMPT  message sent on resume (default: a continue nudge)
#   CLAUDE_LOOP_POLL    seconds to wait when a limit is hit but no reset time is
#                       reported (default: 300)
#   CLAUDE_LOOP_ARGS    extra flags passed to `claude` (e.g.
#                       "--dangerously-skip-permissions" for unattended runs)

sessions_dir="${HOME}/.claude/sessions"
prompt="${CLAUDE_LOOP_PROMPT:-Please continue the task where you left off. When the task is fully complete, stop.}"
poll="${CLAUDE_LOOP_POLL:-300}"
uuid_re='^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'

log() { printf '[claude-loop] %s\n' "$*" >&2; }
die() { log "$*"; exit 1; }

if [[ $# -ne 1 || -z "${1:-}" ]]; then
  die "usage: claude-loop <session-name|session-id>"
fi
query="$1"

# Resolve the argument to a session UUID. A UUID is used as-is; otherwise it's
# treated as a display name and matched against ~/.claude/sessions/*.json,
# preferring the most recently updated match.
if [[ "$query" =~ $uuid_re ]]; then
  session_id="$query"
else
  shopt -s nullglob
  session_files=("$sessions_dir"/*.json)
  shopt -u nullglob
  [[ ${#session_files[@]} -gt 0 ]] || die "no sessions found in $sessions_dir"
  session_id="$(
    jq -rs --arg name "$query" '
      map(select(.name == $name and (.sessionId // "") != ""))
      | sort_by(.updatedAt // 0)
      | last
      | .sessionId // empty
    ' "${session_files[@]}"
  )"
  [[ -n "$session_id" ]] || die "no session named '$query' found in $sessions_dir"
fi

log "resolved session -> $session_id"

# Split optional extra flags into an array so they word-split predictably.
extra_args=()
if [[ -n "${CLAUDE_LOOP_ARGS:-}" ]]; then
  read -ra extra_args <<<"$CLAUDE_LOOP_ARGS"
fi

errfile="$(mktemp)"
trap 'rm -f "$errfile"' EXIT

# Sleep until an absolute epoch (with a small buffer), clamped to a sane range so
# a bogus timestamp can't sleep forever or busy-loop.
sleep_until() {
  local target="$1" now wait
  now="$(date +%s)"
  wait=$(( target - now + 5 ))
  (( wait < 30 )) && wait=30
  (( wait > 21600 )) && wait=21600
  log "usage limit — waiting ${wait}s (until $(date -d "@$target" 2>/dev/null || echo "$target"))"
  sleep "$wait"
}

while true; do
  log "resuming…"
  set +e
  out="$(claude -p --resume "$session_id" --output-format json \
    ${extra_args[@]+"${extra_args[@]}"} "$prompt" 2>"$errfile")"
  rc=$?
  set -e
  err="$(cat "$errfile")"

  # Success path first, so a limit-shaped phrase inside a legitimate result
  # (Claude discussing usage limits) is never mistaken for a real limit.
  if jq -e . >/dev/null 2>&1 <<<"$out"; then
    if [[ "$(jq -r '.is_error' <<<"$out")" == "false" ]]; then
      jq -r '.result // empty' <<<"$out"
      log "session completed its turn — done"
      exit 0
    fi
  elif [[ $rc -eq 0 ]]; then
    # rc 0 but not JSON: treat as a plain successful response.
    printf '%s\n' "$out"
    log "session completed its turn — done"
    exit 0
  fi

  combined="$out $err"

  # Usage/rate limit → wait and probe again.
  if grep -qiE 'usage limit|rate.?limit|too many requests|"?api_error_status"?:? *429' <<<"$combined"; then
    if epoch="$(grep -oE '\b[0-9]{10}\b' <<<"$combined" | head -n1)" && [[ -n "$epoch" ]]; then
      sleep_until "$epoch"
    else
      log "usage limit — no reset time reported, waiting ${poll}s"
      sleep "$poll"
    fi
    continue
  fi

  # Any other error is a genuine failure; don't loop on it.
  log "resume failed (exit $rc), not a usage limit:"
  printf '%s\n' "$combined" >&2
  exit 1
done
