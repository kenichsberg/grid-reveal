## MODIFIED Requirements

### Requirement: Screen has an optional active period

The system SHALL allow administrators to give a screen a start date, an end
date, or both, bounding the days on which progress may be logged against it. The
period SHALL be compared against the caller's calendar day as resolved by the
caller-date capability, never against the server's UTC clock.

#### Scenario: A screen without a period stays open

- **WHEN** a screen has neither a start date nor an end date
- **THEN** it accepts progress indefinitely, exactly as a screen created before periods existed

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

#### Scenario: Administrator edits the period

- **WHEN** an administrator changes a screen's period
- **THEN** the new dates take effect immediately, and extending the end date of a closed screen makes it writable again

## ADDED Requirements

### Requirement: Phase is resolved per caller

The system SHALL resolve a screen's phase independently for each request, from
that caller's calendar day. Two members in different timezones MAY therefore see
the same screen in different phases for part of a day, and each SHALL be judged
by the phase their own day produces.

#### Scenario: Members disagree across a boundary

- **WHEN** two members in timezones a day apart open the same screen on its end date
- **THEN** each sees the phase that their own calendar day produces, and the member still inside the period may write while the other may not

#### Scenario: Phase label matches writability

- **WHEN** a member's screen list reports a screen as active
- **THEN** that member's attempt to log progress against it is not refused as outside the period
