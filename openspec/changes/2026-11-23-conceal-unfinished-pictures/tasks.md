## 1. Withhold the URL at the source

- [x] 1.1 `ScreenSummary.picture_url` becomes `Option<String>` in `apps/api/src/dto.rs`, documented as `None` until every tile is revealed
- [x] 1.2 `to_summary` in `apps/api/src/routes/screens.rs` returns `(ScreenSummary, String)` — the URL beside the summary rather than inside it — because its two callers now answer differently
- [x] 1.3 `list` fills the field only when `revealed_count >= total_tiles`, compared as counts rather than as the rounded percentage the card prints
- [x] 1.4 `directory` passes the URL straight into `AdminScreenEntry`, which keeps its plain `String`
- [x] 1.5 Run `scripts/gen-types.sh`; `picture_url` becomes `string | null` on `ScreenSummary` alone

## 2. Give the card something to show instead

- [x] 2.1 Delete the `.cover` div and its CSS from the `card` snippet in `apps/web/src/routes/+page.svelte`
- [x] 2.2 Render the `<img>` only when `picture_url` is non-null; the server's null is the single source of truth for "finished" and the client does not re-derive it
- [x] 2.3 Fill the thumbnail with `var(--tile)` — the grey `BoardCanvas` paints covered tiles with — so an unfinished card reads as an unopened board rather than a failed image
- [x] 2.4 Centre the percentage in it, large enough to be the card's headline
- [x] 2.5 Hold the headline figure at 99 until the picture arrives: `percent` rounds, so 249 of 250 tiles would otherwise print 100% over a concealed picture

## 3. Say something the card was not already shouting

- [x] 3.1 Add `participant_count` to `ScreenSummary` in `apps/api/src/dto.rs`, filled by the existing `access::participant_count` helper in `to_summary`
- [x] 3.2 Replace the card's `2h · 24/475 tiles · 5%` line with the participant count, worded as `MembersPanel` words it
- [x] 3.3 Drop the now-unused `formatMinutes` import from the page
- [x] 3.4 Stop hiding the headline percentage from screen readers: it was `aria-hidden` only because the meta line repeated it, and that line no longer does

## 4. Take it out of the last two places that served it

- [x] 4.1 Fold the gate into `to_summary`: the picture is now loaded only when `revealed_count >= total_tiles`, so an unfinished screen's URL is never built and cannot leak by being rendered somewhere nobody thought about
- [x] 4.2 Drop `picture_url` from `AdminScreenEntry` and the thumbnail column from the directory list — an administrator is a participant too
- [x] 4.3 Drop `picture_url` from `InvitePeek`; a forwarded link showed the whole picture to someone who had not joined
- [x] 4.4 Give the peek `revealed_count`, `total_tiles` and `participant_count` instead, from the same `access::` helpers the listings use
- [x] 4.5 The invitation page shows the covered board and a line naming the participants and the tiles uncovered; no clamp at 99 here, because there is no picture at any percentage for 100% to be mistaken for
- [x] 4.6 Leave `/admin/pictures` and the clone picker alone: both are choosing a picture, which is the one thing that still needs to see them

## 5. Prove it

- [x] 5.1 Add `apps/api/tests/screen_picture.rs`: a fresh screen's summary carries `picture_url: null` and a finished one carries the URL
- [x] 5.2 Cover the near miss too — one tile short is still short — since that is the case the old fading cover got wrong
- [x] 5.3 Assert the list reports the participant count, which starts at one: the creating administrator is a participant
- [x] 5.4 Assert the directory serves no picture, finished or not, and that a peek serves none either while reporting a participant count that is the screen's own
- [x] 5.5 `cargo test` (159) and `npm run check` both clean
