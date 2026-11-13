## ADDED Requirements

### Requirement: Any member can add a goal

The system SHALL allow any member of a screen to add a free-text goal to that screen.

#### Scenario: Member adds a goal

- **WHEN** a member submits non-empty goal text on a screen they belong to
- **THEN** the system records the goal attributed to that member and displays it in the screen's goal list

#### Scenario: Non-member attempts to add a goal

- **WHEN** an authenticated user who is not a member attempts to add a goal
- **THEN** the system responds 404 and records nothing

#### Scenario: Empty goal rejected

- **WHEN** a member submits empty or whitespace-only goal text
- **THEN** the system rejects the request

### Requirement: Goal content limits

The system SHALL limit goal text to 200 characters and SHALL limit a screen to 50 active goals.

#### Scenario: Text exceeds the limit

- **WHEN** a member submits goal text longer than 200 characters
- **THEN** the system rejects the request and reports the limit

#### Scenario: Screen already at the goal limit

- **WHEN** a member attempts to add a goal to a screen that already has 50 active goals
- **THEN** the system rejects the request and reports that the limit has been reached

### Requirement: Goals are non-interactive text

The system SHALL present goals as plain text bullets with no completion state.

#### Scenario: No completion toggle

- **WHEN** a member views the goal list
- **THEN** goals are displayed as text with no checkbox, no done state, and no way to mark one complete

### Requirement: Goal deletion by author or administrator

The system SHALL allow a goal's author to delete their own goal, and SHALL allow administrators to delete any goal on a screen.

#### Scenario: Author deletes their own goal

- **WHEN** a member deletes a goal they created
- **THEN** the system removes it from the goal list

#### Scenario: Administrator deletes any goal

- **WHEN** an administrator deletes a goal created by another member
- **THEN** the system removes it from the goal list

#### Scenario: Member attempts to delete another member's goal

- **WHEN** a non-administrator member attempts to delete a goal they did not create
- **THEN** the system rejects the request with 403

### Requirement: Goal authorship is visible only to administrators

The system SHALL disclose which member created a goal only to administrators.

#### Scenario: Member sees goals without authors

- **WHEN** a non-administrator member views the goal list
- **THEN** goals appear without any author name, and the API returns no author identity

#### Scenario: Administrator sees authorship

- **WHEN** an administrator views the goal list
- **THEN** each goal shows the username of its author

#### Scenario: Author knows which goals are their own

- **WHEN** a non-administrator member views the goal list
- **THEN** goals they created offer a delete control, which is the only indication of their own authorship

### Requirement: Collapsible goal panel overlays the board

The system SHALL present goals in a panel that overlays the screen's picture and can be folded and unfolded, so the board continues to occupy the full viewport.

#### Scenario: Panel collapsed by default

- **WHEN** a member opens a screen
- **THEN** the goal panel is collapsed to a control showing the goal count, and the board occupies the full viewport

#### Scenario: Panel expanded

- **WHEN** a member expands the goal panel
- **THEN** the goal list and the add-goal control appear overlaying the lower portion of the picture, without pushing the board out of the viewport or introducing page scrolling

#### Scenario: Goals ordered by creation

- **WHEN** the goal list is displayed
- **THEN** goals appear in the order they were created

### Requirement: Goals on archived screens are read-only

The system SHALL reject goal creation and deletion on an archived screen.

#### Scenario: Adding a goal to an archived screen

- **WHEN** a member attempts to add or delete a goal on an archived screen
- **THEN** the system rejects the request and reports that the screen is archived

#### Scenario: Existing goals remain visible

- **WHEN** a member views an archived screen
- **THEN** its goals remain readable in the panel
