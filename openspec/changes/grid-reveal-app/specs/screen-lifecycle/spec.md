## ADDED Requirements

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

### Requirement: Administrator unarchives a screen

The system SHALL allow administrators to return an archived screen to active status.

#### Scenario: Unarchiving restores writability

- **WHEN** an administrator unarchives a screen
- **THEN** the screen returns to the active section of members' screen lists and accepts entries and goals again

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
