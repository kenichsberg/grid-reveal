## ADDED Requirements

### Requirement: Member logs progress

The system SHALL allow a member of a screen to log a duration of time against that screen, entered as hours and minutes.

#### Scenario: Successful entry

- **WHEN** a member submits hours and minutes totalling at least 1 minute on a screen they belong to
- **THEN** the system records an entry attributed to that member with the duration stored as whole minutes

#### Scenario: Non-member attempts to log progress

- **WHEN** an authenticated user who is not a member attempts to log progress on a screen
- **THEN** the system responds 404 and records nothing

#### Scenario: Zero duration rejected

- **WHEN** a member submits 0 hours and 0 minutes
- **THEN** the system rejects the entry and reports that at least 1 minute is required

### Requirement: Duration input bounds

The system SHALL accept hours in the range 0 to 23 and minutes in the range 0 to 59 for a single entry, storing the combined value as an integer count of minutes.

#### Scenario: Maximum single entry

- **WHEN** a member submits 23 hours and 59 minutes
- **THEN** the system records an entry of 1439 minutes

#### Scenario: Out-of-range input rejected

- **WHEN** a member submits hours above 23 or minutes above 59
- **THEN** the system rejects the entry and reports the accepted ranges

### Requirement: Entry date selection

The system SHALL store a date for each entry, defaulting to today and selectable by the member, and SHALL confine it to the past and to the screen's active period. The date SHALL be stored as a calendar date with no time component or timezone conversion.

#### Scenario: Default date

- **WHEN** a member opens the progress entry form
- **THEN** today's date is pre-selected

#### Scenario: Backdating an entry

- **WHEN** a member selects an earlier date and submits the entry
- **THEN** the system stores that date and the entry appears under it in the member's history

#### Scenario: Future date rejected

- **WHEN** a member selects a date later than today
- **THEN** the system rejects the entry and reports that future dates are not accepted

#### Scenario: Date outside the screen's period rejected

- **WHEN** a member selects a date before the screen's start date or after its end date
- **THEN** the system rejects the entry and reports that the date falls outside the screen's period

#### Scenario: Entry form bounds the date picker

- **WHEN** a member opens the entry form on a screen with a period
- **THEN** the date field offers only dates within that period and no later than today

### Requirement: Entry ordering for attribution

The system SHALL determine which entry revealed which tile by replaying entries in order of their creation timestamp, not their selected date.

#### Scenario: Backdating does not reshuffle attribution

- **WHEN** a member records an entry dated earlier than existing entries
- **THEN** the entry is appended for attribution purposes and the ownership of previously revealed tiles is unchanged

#### Scenario: History displays by selected date

- **WHEN** a member views their entry history
- **THEN** entries are presented ordered by their selected date, independently of attribution order

### Requirement: Member edits own entry

The system SHALL allow a member to edit the duration and date of their own entries.

#### Scenario: Correcting a mistyped duration

- **WHEN** a member edits one of their entries to a different valid duration
- **THEN** the system updates the entry, records an edit timestamp, and the screen total and revealed tile count recompute accordingly

#### Scenario: Editing another member's entry

- **WHEN** a member attempts to edit an entry belonging to someone else
- **THEN** the system rejects the request with 403

### Requirement: Member deletes own entry

The system SHALL allow a member to delete their own entries. Deletion SHALL be a soft delete that preserves the row for audit while excluding it from all totals.

#### Scenario: Deleting an entry rewinds the board

- **WHEN** a member deletes an entry of 10 minutes on a screen with a 5-minute unit
- **THEN** the screen total decreases by 10 minutes and two tiles become unrevealed again

#### Scenario: Deleted entry is retained but excluded

- **WHEN** an entry is deleted
- **THEN** the system marks it deleted rather than removing the row, and excludes it from every total and from attribution

#### Scenario: Deleted entry leaves the member's history

- **WHEN** a member deletes an entry and reopens their history
- **THEN** the deleted entry is no longer listed

### Requirement: Private per-member history

The system SHALL show each member only their own entry history on a screen.

#### Scenario: Member views their own history

- **WHEN** a member opens the history view for a screen
- **THEN** the system lists only entries belonging to that member

#### Scenario: Member cannot see another member's entries

- **WHEN** a member requests entry data for a screen
- **THEN** the system returns no entries belonging to other members and exposes no per-member breakdown or leaderboard

### Requirement: Aggregate screen total is shared

The system SHALL display the combined total logged time across all members to every member of the screen.

#### Scenario: Total reflects all members

- **WHEN** a member views a screen on which several members have logged time
- **THEN** the system displays the combined total of all non-deleted entries without attributing any portion to a named member

### Requirement: Logging on completed, closed, and archived screens

The system SHALL continue accepting entries on a fully revealed screen, and SHALL reject entries on a screen that is archived, has not started, or whose period has ended.

#### Scenario: Logging after full reveal

- **WHEN** a member logs time on a screen where every tile is already revealed
- **THEN** the system records the entry and adds it to the screen total, while the revealed tile count remains at its maximum

#### Scenario: Logging on an archived screen

- **WHEN** a member attempts to log, edit, or delete an entry on an archived screen
- **THEN** the system rejects the request and reports that the screen is archived

#### Scenario: Logging on a screen that has not started

- **WHEN** a member attempts to log, edit, or delete an entry before the screen's start date
- **THEN** the system rejects the request and reports that the period has not started

#### Scenario: Logging after the period ends

- **WHEN** a member attempts to log, edit, or delete an entry after the screen's end date
- **THEN** the system rejects the request and reports that the period has ended

### Requirement: No logging on behalf of another user

The system SHALL attribute every entry to the authenticated user who submitted it, with no mechanism for logging on another user's behalf.

#### Scenario: Administrator cannot log for a member

- **WHEN** an administrator submits a progress entry on a screen
- **THEN** the entry is attributed to the administrator and the API accepts no target-user parameter
