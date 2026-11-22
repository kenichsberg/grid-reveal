## RENAMED Requirements

- FROM: `### Requirement: Screen has an optional active period`
- TO: `### Requirement: Screen has a required active period`

## MODIFIED Requirements

### Requirement: Screen has a required active period

The system SHALL require every screen to have both a start date and an end
date, bounding the days on which progress may be logged against it. Neither
bound SHALL be absent: a screen cannot be created, cloned, or edited into a
state without both. The period SHALL be compared against the caller's calendar
day as resolved by the caller-date capability, never against the server's UTC
clock.

#### Scenario: Before the period opens

- **WHEN** a member opens a screen whose start date has not arrived
- **THEN** the board, goals, and member list are displayed, the screen reports that it has not started, and any attempt to add, edit, or delete an entry or a goal is rejected

#### Scenario: Within the period

- **WHEN** the caller's calendar day falls on or between the start and end dates
- **THEN** the screen accepts entries and goals

#### Scenario: First day of the period, after local midnight

- **WHEN** a member whose local day is ahead of the server's UTC day opens a screen whose start date is that local day
- **THEN** the screen accepts entries and goals, rather than reporting that its period has not started

#### Scenario: Last day of the period, during local evening

- **WHEN** a member whose local day is behind the server's UTC day logs progress on the screen's end date
- **THEN** the screen accepts the entry, rather than reporting that its period has ended

#### Scenario: End date cannot precede the start date

- **WHEN** an administrator submits a period whose end date falls before its start date
- **THEN** the system rejects the request and reports that the end date cannot fall before the start date

#### Scenario: A bound cannot be removed

- **WHEN** an administrator submits a period with a missing start date or end date
- **THEN** the system rejects the request and names the missing date, rather than storing a screen with no bound

#### Scenario: Administrator edits the period

- **WHEN** an administrator changes a screen's period
- **THEN** the new dates take effect immediately, and extending the end date of a closed screen makes it writable again

### Requirement: Administrator clones a screen

The system SHALL allow administrators, and only administrators, to clone an existing screen into a new screen with a fresh, fully unrevealed board.

#### Scenario: Clone copies configuration

- **WHEN** an administrator clones a screen
- **THEN** the new screen copies `cols`, `rows`, and `unit_minutes` from the source

#### Scenario: Clone copies members and goals

- **WHEN** an administrator clones a screen
- **THEN** the new screen receives the same member list and a copy of the source's active goals

#### Scenario: Clone starts unrevealed

- **WHEN** a clone is created
- **THEN** it has no progress entries, a newly generated permutation, a revealed count of zero, and its own unique slug

#### Scenario: Clone does not inherit the period

- **WHEN** an administrator clones a screen
- **THEN** the new screen takes the period the administrator supplies rather than the source's, because a clone is a new period rather than a copy

#### Scenario: Clone requires a period

- **WHEN** an administrator clones a screen without supplying both dates
- **THEN** the system rejects the request, since a clone is a screen and every screen has a period

#### Scenario: Source screen is unaffected

- **WHEN** a clone is created
- **THEN** the source screen's entries, goals, members, and revealed state are unchanged

#### Scenario: Non-administrator attempts to clone

- **WHEN** a non-administrator attempts to clone a screen
- **THEN** the system rejects the request with 403
