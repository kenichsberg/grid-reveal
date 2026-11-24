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
- The invitation page stops showing the picture too, and
  `InvitePeek.picture_url` goes with it. A link forwarded to anyone at all
  showed the whole of what the group was uncovering, to someone who had not
  even joined. In its place the peek carries where the board stands and how
  many people are on it, which is what an invitee actually wants to know.
- The administrator directory stops showing thumbnails, and
  `AdminScreenEntry.picture_url` goes. An administrator is a participant too —
  the creating one is a member by default — so a wall of pictures spoils every
  board they are on.
- After this there are exactly two places a picture is served: choosing one,
  and a board whose last tile has fallen.
- The line beneath the card's title stops repeating the board. It read
  `2h · 24/475 tiles · 5%`, two thirds of which the card now prints in its own
  mouth at four times the size; it becomes the participant count, which is the
  one thing about a screen the list could not say before. `ScreenSummary` gains
  `participant_count` to carry it.

## Non-goals

- The board page. A partially covered picture there is the product.
- The picture library (`/admin/pictures`) and the clone picker, which are both
  an administrator choosing a picture. A library of dimensions with no
  thumbnails could not be used to pick, hide or delete anything.
- The board itself, where a partially covered picture is the product.
