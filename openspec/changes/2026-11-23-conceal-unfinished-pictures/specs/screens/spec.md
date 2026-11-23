## MODIFIED Requirements

### Requirement: Member-scoped screen list

The system SHALL provide each authenticated user a list of exactly the screens they are a member of, separating active from archived screens, SHALL withhold a screen's background picture from that list until every tile of its board is revealed, and SHALL report how many participants each screen has.

#### Scenario: User views their screen list

- **WHEN** an authenticated user opens their screen list
- **THEN** the system lists only screens where they are a member, and lists no others

#### Scenario: An administrator's personal list is still member-scoped

- **WHEN** an administrator opens their own screen list
- **THEN** the system lists only screens they are a member of, not every screen; the full set is reached through the administrator screen directory instead

#### Scenario: Archived screens are separated

- **WHEN** a user's screen list contains archived screens
- **THEN** the system presents them in a section distinct from active screens

#### Scenario: An unfinished board discloses no picture

- **WHEN** a user's screen list contains a screen whose revealed count is below its tile count
- **THEN** the summary for that screen carries no picture URL, so the browser cannot fetch the picture from this page at all

#### Scenario: One tile short is still short

- **WHEN** a screen stands one tile away from completion
- **THEN** its summary still carries no picture URL, regardless of how the progress percentage rounds

#### Scenario: A finished board shows its picture

- **WHEN** every tile of a screen is revealed
- **THEN** its summary carries the picture URL and the list shows the picture

#### Scenario: Concealment survives the end of the period

- **WHEN** a screen's period ends, or it is archived, with tiles still covered
- **THEN** its summary continues to carry no picture URL

#### Scenario: The list reports how many people are on a screen

- **WHEN** a user opens their screen list
- **THEN** each screen reports its participant count, and the card shows it rather than restating the progress figure it already displays

#### Scenario: The administrator directory is unaffected

- **WHEN** an administrator opens the screen directory
- **THEN** every entry carries its picture URL whatever its revealed count, since administrators choose the pictures
