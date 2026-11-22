# Reveal Board

## Purpose

How logged time becomes a picture. The board is derived from the screen's
total minutes and a fixed permutation, never stored, and the client is sent
only the tiles already revealed so the reveal order cannot be read ahead.

## Requirements

### Requirement: Revealed tiles are derived, not stored

The system SHALL compute the set of revealed tiles from the screen's total logged time and its fixed tile permutation, and SHALL NOT store individual tile removal events.

#### Scenario: Revealed count derived from the total

- **WHEN** the system renders or serves a screen's board state
- **THEN** the revealed count equals `LEAST(floor(total_minutes / unit_minutes), cols × rows)` where `total_minutes` is the sum of all non-deleted entries

#### Scenario: Revealed tiles are the permutation prefix

- **WHEN** the revealed count is `n`
- **THEN** the revealed tiles are exactly the first `n` entries of the screen's permutation, in order

#### Scenario: Partial unit does not reveal a tile

- **WHEN** a screen with a 5-minute unit has a total of 13 minutes
- **THEN** exactly 2 tiles are revealed and 3 minutes carry toward the next tile

#### Scenario: Another member's minutes complete a partial unit

- **WHEN** a screen stands at 13 minutes with a 5-minute unit and any member logs 2 further minutes
- **THEN** the total becomes 15 and a third tile is revealed

### Requirement: Fixed tile permutation per screen

The system SHALL generate a random permutation of all tile indices once when a screen's geometry is established, and SHALL use it as the reveal order until the geometry changes.

#### Scenario: Permutation generated at creation

- **WHEN** a screen is created with `cols × rows` tiles
- **THEN** the system stores a random permutation containing every tile index exactly once

#### Scenario: Reveal order is stable

- **WHEN** entries are added to a screen over time without a geometry change
- **THEN** tiles are revealed in the stored permutation order every time, and re-rendering the board produces the identical tile set

### Requirement: Only the revealed prefix is sent to the client

The system SHALL send clients only the already-revealed portion of the permutation.

#### Scenario: Unrevealed order is withheld

- **WHEN** a client fetches a screen's board state
- **THEN** the response contains the revealed tile indices and the revealed count, and contains no information about which tiles will be revealed next

### Requirement: Tile attribution is recorded but not displayed

The system SHALL be able to determine which member's entry revealed each tile, and SHALL NOT display that attribution to members in this release.

#### Scenario: Attribution is derivable

- **WHEN** entries are replayed in creation order
- **THEN** each revealed tile can be attributed to the entry whose cumulative total crossed that tile's threshold

#### Scenario: Attribution is absent from member-facing views

- **WHEN** a member views the board
- **THEN** no tile indicates who revealed it, and the API returns no per-tile attribution to non-administrators

### Requirement: Board rendering fills the board area

The system SHALL display the background picture scaled to fit entirely within the board area — the viewport less the screen's own chrome — with the tile grid overlaying exactly the picture's area, and SHALL never introduce page scrolling.

#### Scenario: Picture letterboxed to fit

- **WHEN** a screen is displayed in a board area whose aspect ratio differs from the picture's
- **THEN** the picture is scaled to fit entirely within the board area and the remaining area is letterboxed

#### Scenario: Grid aligns to the picture

- **WHEN** the picture is rendered
- **THEN** the tile grid covers exactly the picture's area, with each cell measuring `picture_width / cols` by `picture_height / rows`

#### Scenario: Viewport resize

- **WHEN** the browser window is resized
- **THEN** the picture and grid rescale together and remain aligned

#### Scenario: Board area shrinks when a tab panel opens

- **WHEN** a member selects a tab that occupies part of the board area on a wide viewport
- **THEN** the picture re-letterboxes into the smaller area and stays aligned with its grid

### Requirement: Board chrome is reserved, not overlaid

The system SHALL reserve the screen's chrome outside the board area rather than floating it over the picture, so that no control lands on the photograph whatever its aspect ratio.

#### Scenario: Narrow viewport

- **WHEN** a member opens a screen on a narrow viewport
- **THEN** a band above the board carries the title, the current and goal hours, a progress meter, and the period, and a tab bar below it carries the board, record, goals, and member tabs

#### Scenario: Current and goal hours

- **WHEN** the band is displayed
- **THEN** it shows the hours logged by everyone on the screen as "current", and the hours needed to reveal every tile as "goal", both in hours

#### Scenario: Wide viewport

- **WHEN** a member opens a screen on a wide viewport
- **THEN** the title, hours, meter, and period move to a rail beside the board, the tabs move to a strip above it, and a selected tab's content takes a column beside the board rather than covering it

#### Scenario: Primary action placement

- **WHEN** progress may be logged
- **THEN** the control that opens the entry form is centred above the tab bar on a narrow viewport and placed in the rail on a wide one

#### Scenario: Primary action withheld

- **WHEN** progress may not be logged because the screen has not started, its period has ended, or it is archived
- **THEN** the control is absent and the reason is stated in its place

### Requirement: Cover tile appearance

The system SHALL draw unrevealed tiles as flat gray cells with a border drawn inside each cell, so that a revealed tile leaves no visible trace.

#### Scenario: Unrevealed tiles are opaque

- **WHEN** a tile is unrevealed
- **THEN** it is drawn as a flat gray cell that fully conceals the picture beneath it

#### Scenario: Revealed tile leaves no grid line

- **WHEN** a tile is revealed
- **THEN** nothing is drawn in its area and the picture shows through cleanly, with any boundary line remaining only on the adjacent covered tile

### Requirement: Canvas rendering

The system SHALL render the tile grid on a 2D canvas, redrawing only while an animation is in progress.

#### Scenario: Large grid renders

- **WHEN** a screen has several thousand tiles
- **THEN** the board renders without creating one DOM element per tile

#### Scenario: Static board does no work

- **WHEN** the board is displayed and no animation is running
- **THEN** no redraw occurs

### Requirement: Reveal animation is shown only to the submitting member

The system SHALL animate newly revealed tiles only for the member whose entry revealed them, and SHALL apply other members' progress without animation.

#### Scenario: Submitter sees only their own tiles animate

- **WHEN** a client showing 10 revealed tiles submits an entry, and the server reports 17 revealed because another member had meanwhile reached 15
- **THEN** the client applies tiles 10 through 14 instantly without animation, then animates only tiles 15 and 16

#### Scenario: Submit response identifies the submitter's tiles

- **WHEN** an entry is accepted
- **THEN** the response includes the new revealed count and the tile indices attributable to that submission

#### Scenario: No animation on page load

- **WHEN** a member loads a screen, regardless of how much progress occurred since their last visit
- **THEN** the board renders at its current state with no reveal animation

#### Scenario: Other members are not notified in real time

- **WHEN** one member submits an entry
- **THEN** other members with the screen open see no change until they reload

### Requirement: Completion celebration

The system SHALL celebrate a fully revealed screen for every member on every visit, while continuing to accept and total further entries.

#### Scenario: Celebration on the completing submission

- **WHEN** a member's entry reveals the final tile
- **THEN** the reveal animation completes and the celebration is presented

#### Scenario: Celebration on every subsequent visit

- **WHEN** any member loads a screen on which every tile is already revealed
- **THEN** the celebration is presented again

#### Scenario: Celebration can be dismissed for the visit

- **WHEN** a member dismisses the celebration
- **THEN** it does not reappear during that visit, and the fully revealed picture and total time remain visible

#### Scenario: Overflow time is counted

- **WHEN** members continue logging time on a completed screen
- **THEN** the displayed total increases while the revealed count stays at `cols × rows`

### Requirement: Geometry change recomputes the board

The system SHALL regenerate the permutation and recompute the revealed count when a screen's `cols`, `rows`, or `unit_minutes` change, without altering any entry.

#### Scenario: New permutation after a geometry change

- **WHEN** a screen's `cols` or `rows` change
- **THEN** the system generates a new permutation covering the new tile set

#### Scenario: Revealed count recomputed

- **WHEN** a screen's `unit_minutes` increases such that fewer tiles are warranted
- **THEN** the revealed count decreases accordingly and part of the picture becomes covered again

#### Scenario: Total time is preserved

- **WHEN** any geometry change is applied
- **THEN** the screen's total logged time is identical before and after
