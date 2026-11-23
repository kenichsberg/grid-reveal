## Why

The screen list renders every screen's picture and lays a flat tile-coloured
cover over it at `opacity: 1 - pct/100`. The intent was for a card to echo its
board, but a uniform fade is not a grid of tiles: a cover at 60% opacity is a
photograph seen through gauze, and by the time a group is halfway through the
work, anyone opening their own list has already seen what they were working
towards.

That is the one secret the application keeps. The board goes to the trouble of
sending only the revealed *prefix* of the permutation, so that a member cannot
read the reveal order ahead — and then the lobby hands over the picture itself,
whole, to a browser that only has to set `opacity: 0`.

The fix is the one already used for tile order: the server does not send what
the member has not earned.

## What Changes

- `ScreenSummary.picture_url` becomes `Option<String>`, populated only when
  `revealed_count >= total_tiles`. The browser never fetches the picture from
  the list page of an unfinished screen.
- The card drops its `.cover` overlay. An unfinished screen is a flat panel in
  the board's own tile grey with the percentage in it; the picture appears only
  when the last tile falls, where it reads as the reward it is.
- `AdminScreenEntry.picture_url` is untouched. Administrators choose the
  pictures, so the directory has nothing to spoil and every reason to show
  which picture a screen uses.
- The line beneath the card's title stops repeating the board. It read
  `2h · 24/475 tiles · 5%`, two thirds of which the card now prints in its own
  mouth at four times the size; it becomes the participant count, which is the
  one thing about a screen the list could not say before. `ScreenSummary` gains
  `participant_count` to carry it.

## Non-goals

- The board page. A partially covered picture there is the product.
- The invite page (`/join/<token>`), which still shows the picture uncovered to
  anyone holding a link. The same leak, but it is load-bearing: the picture is
  part of what an invitee is deciding to join. Worth revisiting on its own.
