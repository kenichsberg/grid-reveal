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

### Requirement: Goals are reached from the board's tab bar

The system SHALL present goals as one of the board's tabs, alongside the board itself, the member's own record, and the member list.

#### Scenario: Board shown by default

- **WHEN** a member opens a screen
- **THEN** the board tab is selected and the picture occupies the board area

#### Scenario: Goals tab selected

- **WHEN** a member selects the Goals tab
- **THEN** the goal list and the add-goal control replace the board area on a narrow viewport, or appear beside it on a wide one, without introducing page scrolling

#### Scenario: Selected tab survives reload and back

- **WHEN** a member selects the Goals tab and then reloads, or navigates back
- **THEN** reloading returns to the Goals tab and one back press returns to the board without leaving the screen

#### Scenario: Goals ordered by creation

- **WHEN** the goal list is displayed
- **THEN** goals appear in the order they were created

### Requirement: Goals freeze when a screen closes

The system SHALL reject goal creation and deletion on a screen that is archived, has not started, or whose period has ended.

#### Scenario: Adding a goal to an archived screen

- **WHEN** a member attempts to add or delete a goal on an archived screen
- **THEN** the system rejects the request and reports that the screen is archived

#### Scenario: Adding a goal after the period ends

- **WHEN** a member attempts to add or delete a goal on a screen whose period has ended
- **THEN** the system rejects the request and reports that the period has ended

#### Scenario: Existing goals remain visible

- **WHEN** a member views an archived or closed screen
- **THEN** its goals remain readable in the Goals tab, without the add or delete controls
