# Screen Lifecycle

## Purpose

What happens to a screen over time: the period it accepts progress in, the
read-only state that follows, cloning it into a fresh run, archiving it by
hand or after a retention term, and destroying it.

## Requirements

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

#### Scenario: Clone does not inherit invites

- **WHEN** an administrator clones a screen that has pending invites
- **THEN** the clone has none, and a link issued for the source screen never opens the clone

#### Scenario: Source screen is unaffected

- **WHEN** a clone is created
- **THEN** the source screen's entries, goals, members, and revealed state are unchanged

#### Scenario: Non-administrator attempts to clone

- **WHEN** a non-administrator attempts to clone a screen
- **THEN** the system rejects the request with 403

### Requirement: Clone requires a new title and a new picture

The system SHALL require the administrator to supply a new title and select or upload a picture when cloning, rather than inheriting the source screen's title or picture.

#### Scenario: Title required on clone

- **WHEN** an administrator attempts to clone without supplying a title
- **THEN** the system rejects the request and reports that a title is required

#### Scenario: Picture required on clone

- **WHEN** an administrator attempts to clone without uploading or selecting a picture
- **THEN** the system rejects the request and reports that a picture is required

#### Scenario: Clone may reuse a library picture

- **WHEN** an administrator selects the source screen's picture from the library while cloning
- **THEN** the system accepts it, and both screens reference the same stored picture

### Requirement: Administrator prunes a clone's goals

The system SHALL allow an administrator to delete any of the goals copied into a clone.

#### Scenario: Pruning copied goals

- **WHEN** an administrator deletes goals on a newly created clone
- **THEN** those goals are removed from the clone and the source screen's goals are unaffected

### Requirement: Administrator archives a screen

The system SHALL allow administrators to archive a screen, making it read-only while keeping it viewable by its members.

#### Scenario: Archiving a screen

- **WHEN** an administrator archives a screen
- **THEN** the screen is marked archived and moves to the archived section of each member's screen list

#### Scenario: Archived screen is viewable

- **WHEN** a member opens an archived screen
- **THEN** the board, total logged time, and goals are displayed as they stood at archival

#### Scenario: Archived screen rejects modification

- **WHEN** any member or administrator attempts to add, edit, or delete an entry or a goal on an archived screen
- **THEN** the system rejects the request and reports that the screen is archived

#### Scenario: Non-administrator attempts to archive

- **WHEN** a non-administrator attempts to archive a screen
- **THEN** the system rejects the request with 403

#### Scenario: Archiving a screen whose period is still open

- **WHEN** an administrator archives a screen before its end date
- **THEN** the screen becomes read-only immediately, regardless of its period

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

### Requirement: Closed period makes a screen read-only

The system SHALL freeze a screen once its end date has passed, keeping the board and any completion celebration visible while refusing every change.

#### Scenario: Entries are frozen

- **WHEN** a member attempts to add, edit, or delete an entry on a screen whose period has ended
- **THEN** the system rejects the request and reports that the period has ended

#### Scenario: Goals are frozen

- **WHEN** a member attempts to add or delete a goal on a screen whose period has ended
- **THEN** the system rejects the request and reports that the period has ended

#### Scenario: Board remains visible

- **WHEN** a member opens a screen whose period has ended
- **THEN** the revealed tiles, total logged time, goals, and member list are displayed as they stood when the period closed

#### Scenario: Completion is still celebrated

- **WHEN** a member opens a fully revealed screen whose period has ended
- **THEN** the completion celebration is shown

#### Scenario: Ended screen stays in the active list

- **WHEN** a screen's period has ended but it has not been archived
- **THEN** it remains in the active section of each member's screen list, labelled with the date it ended

### Requirement: Screen archives itself after a retention term

The system SHALL archive a screen automatically once an administrator-set number of days has passed since its end date, so that a finished period retires without an administrator having to remember it.

#### Scenario: Setting a retention term

- **WHEN** an administrator sets a period with an end date
- **THEN** they may also set the number of days the screen stays visible after that date before archiving itself, leaving it unset to mean never

#### Scenario: Retention term expires

- **WHEN** more days have passed since a screen's end date than its retention term allows
- **THEN** the screen reports itself as archived and appears in the archived section of each member's screen list

#### Scenario: Retention term requires an end date

- **WHEN** an administrator sets a retention term on a screen with no end date
- **THEN** the system rejects the request and reports that the term needs an end date to count from

#### Scenario: Retention term is not negative

- **WHEN** an administrator sets a negative retention term
- **THEN** the system rejects the request

### Requirement: Administrator unarchives a screen

The system SHALL allow administrators to return an archived screen to active status.

#### Scenario: Unarchiving restores writability

- **WHEN** an administrator unarchives a screen whose period is still open
- **THEN** the screen returns to the active section of members' screen lists and accepts entries and goals again

#### Scenario: Unarchiving cancels the automatic archive

- **WHEN** an administrator unarchives a screen that archived itself when its retention term expired
- **THEN** the retention term is cleared, so the screen does not archive itself again

#### Scenario: Unarchiving does not reopen a closed period

- **WHEN** an administrator unarchives a screen whose end date has passed
- **THEN** the screen returns to the active list but stays read-only until the period is extended

### Requirement: Screen deletion requires explicit confirmation

The system SHALL allow administrators to permanently delete a screen only after confirming by typing the screen's exact title.

#### Scenario: Deletion confirmed by typing the title

- **WHEN** an administrator requests deletion and types the screen's title exactly
- **THEN** the system deletes the screen together with its entries, goals, and membership rows

#### Scenario: Confirmation text does not match

- **WHEN** an administrator types text that does not exactly match the screen's title
- **THEN** the system refuses to delete and the screen remains unchanged

#### Scenario: Picture is retained

- **WHEN** a screen is deleted
- **THEN** its background picture remains in the shared library and is not removed from object storage

#### Scenario: Non-administrator attempts deletion

- **WHEN** a non-administrator attempts to delete a screen
- **THEN** the system rejects the request with 403
