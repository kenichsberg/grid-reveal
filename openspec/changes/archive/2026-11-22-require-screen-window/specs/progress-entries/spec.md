## MODIFIED Requirements

### Requirement: Entry date selection

The system SHALL store a date for each entry, defaulting to today and selectable
by the member, and SHALL confine it to the past and to the screen's active
period. The date SHALL be stored as a calendar date with no time component or
timezone conversion. "Today" SHALL mean the caller's calendar day as resolved by
the caller-date capability, never the day on the server's UTC clock. Every
screen has a period, so every entry is bounded at both ends.

#### Scenario: Default date

- **WHEN** a member opens the progress entry form
- **THEN** their own calendar day is pre-selected

#### Scenario: Backdating an entry

- **WHEN** a member selects an earlier date and submits the entry
- **THEN** the system stores that date and the entry appears under it in the member's history

#### Scenario: Future date rejected

- **WHEN** a member selects a date later than their own calendar day
- **THEN** the system rejects the entry and reports that future dates are not accepted

#### Scenario: Logging after local midnight

- **WHEN** a member whose local day is ahead of the server's UTC day logs progress for their current day
- **THEN** the system accepts the entry and stores it under the member's day, rather than refusing it as a future date

#### Scenario: Date outside the screen's period rejected

- **WHEN** a member selects a date before the screen's start date or after its end date
- **THEN** the system rejects the entry and reports that the date falls outside the screen's period

#### Scenario: Entry form bounds the date picker

- **WHEN** a member opens the entry form on any screen
- **THEN** the date field offers only dates within that screen's period and no later than the member's own calendar day
