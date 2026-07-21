Run the daily Anki progress tracker — data-only bookkeeping, no coaching or next-session recommendations. Use the sensei skill conventions and the anki-tool CLI.

Steps:

1. anki-tool sync
2. anki-tool overview and anki-tool session "日本語"
3. Append a concise data-only entry to the "Log / milestones" section, dated per the Date nuance below. This is the home for all daily/volatile numbers: reviews done, good/easy/again/hard counts, victories count, total due, and any notable lapsed-card cluster.
4. Only touch the "Current level" section if the standing state actually changed. It is a single current-state view, NOT an append-only log — do not add a new dated snapshot each run. If standing figures (e.g. deck mature/young ratios, new-cards remaining) have drifted, edit the most recent snapshot in place. Start a new dated snapshot only when something qualitative is worth preserving (a lesson unlocked, a leech finally cleared, a structural change). Never put daily flow numbers like "reviewed today" here — those belong in the Log entry from step 3.

Do NOT invent coaching or drills. Keep the note pruned per CLAUDE.md (update/append, no duplicate sections). Update the memory pointer japanese_study.md only if a number materially changed.

Date nuance: Anki's study day rolls over at 4am, so date the log entry to the study day the counts actually cover, which is not necessarily the wall-clock date. If this job fired between midnight and 4am, reviewed_today still reflects the previous calendar date — use that; if it fired at or after 4am, use today.

Commit and push (this vault is pre-authorized); if push is rejected, git pull --rebase and retry. Fix merge conflicts on your own.
