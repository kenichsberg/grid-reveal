## MODIFIED Requirements

### Requirement: Administrator creates a screen

The system SHALL allow administrators, and only administrators, to create screens. A screen SHALL have a title, a background picture, a grid geometry, a unit of time per tile, and an active period with both a start date and an end date.

#### Scenario: Successful creation

- **WHEN** an administrator supplies a title, a picture, a target total time, a unit time per tile, and a period
- **THEN** the system creates the screen with a derived `cols × rows` geometry, a random slug, a fixed tile permutation, and the creating administrator as a member

#### Scenario: Period set at creation

- **WHEN** an administrator supplies a start date and an end date at creation
- **THEN** the system stores them, and offers a default retention term after which the screen archives itself

#### Scenario: Start date defaults to the administrator's day

- **WHEN** an administrator opens the screen creation form
- **THEN** the start date is pre-filled with their own calendar day, leaving the end date as the only date they must decide

#### Scenario: Period omitted at creation

- **WHEN** an administrator attempts to create a screen without both dates
- **THEN** the system rejects the request and names the missing date

#### Scenario: Title is required

- **WHEN** an administrator attempts to create a screen with an empty or whitespace-only title
- **THEN** the system rejects the request and reports that a title is required

### Requirement: Administrator edits a screen

The system SHALL allow administrators to edit a screen's title, its active period, and its `cols`, `rows`, and `unit_minutes` after entries exist.

#### Scenario: Title edited

- **WHEN** an administrator changes a screen's title to a non-empty value
- **THEN** the system saves it and the new title appears in the screen header and the screen list

#### Scenario: Period edited

- **WHEN** an administrator submits a screen's period
- **THEN** the system replaces the stored period wholesale with the two dates supplied

#### Scenario: Period left alone

- **WHEN** an administrator submits an edit that carries no period at all
- **THEN** the stored period is untouched, which is how a title or geometry is changed without restating the dates

#### Scenario: Period cannot be cleared

- **WHEN** an administrator submits a period with either date missing or null
- **THEN** the system rejects the request and names the missing date, rather than clearing that bound

#### Scenario: Period edit is allowed on a closed screen

- **WHEN** an administrator edits the period of a screen whose period has ended or which is archived
- **THEN** the system accepts the edit, since extending the period is how such a screen is reopened

#### Scenario: Geometry edit warns before confirmation

- **WHEN** an administrator changes `cols`, `rows`, or `unit_minutes` on a screen that has entries
- **THEN** the system shows the resulting tile count and revealed tile count before confirmation, warning that the picture may become less revealed and that the reveal order will change

#### Scenario: Logged time survives a geometry edit

- **WHEN** a geometry edit is confirmed
- **THEN** no entry is modified or deleted and the screen's total logged time is unchanged
