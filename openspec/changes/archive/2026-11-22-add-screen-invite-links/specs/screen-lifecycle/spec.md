## MODIFIED Requirements

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
