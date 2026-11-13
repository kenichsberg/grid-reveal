## ADDED Requirements

### Requirement: Administrator creates a screen

The system SHALL allow administrators, and only administrators, to create screens. A screen SHALL have a title, a background picture, a grid geometry, and a unit of time per tile.

#### Scenario: Successful creation

- **WHEN** an administrator supplies a title, a picture, a target total time, and a unit time per tile
- **THEN** the system creates the screen with a derived `cols × rows` geometry, a random slug, a fixed tile permutation, and the creating administrator as a member

#### Scenario: Title is required

- **WHEN** an administrator attempts to create a screen with an empty or whitespace-only title
- **THEN** the system rejects the request and reports that a title is required

#### Scenario: Non-administrator attempts creation

- **WHEN** an authenticated non-administrator attempts to create a screen
- **THEN** the system rejects the request with 403

### Requirement: Picture selection at creation

The system SHALL let an administrator either upload a new picture or select an existing one from the shared library when creating a screen.

#### Scenario: Creating with a newly uploaded picture

- **WHEN** an administrator uploads a picture during screen creation
- **THEN** the system processes and adds it to the shared library, then attaches it to the new screen

#### Scenario: Creating with a library picture

- **WHEN** an administrator selects an existing library picture during screen creation
- **THEN** the system attaches that picture to the new screen without duplicating the stored object

#### Scenario: No picture selected

- **WHEN** an administrator attempts to create a screen without uploading or selecting a picture
- **THEN** the system rejects the request and reports that a picture is required

### Requirement: Grid geometry derived from picture aspect ratio

The system SHALL propose a `cols × rows` geometry whose ratio most closely matches the selected picture's aspect ratio for the requested tile count, and SHALL report the exact total time the proposed geometry yields.

#### Scenario: Geometry proposed from target and unit

- **WHEN** an administrator enters a target total time and a unit time per tile
- **THEN** the system computes an approximate tile count and proposes a `cols × rows` whose ratio is closest to the picture's aspect ratio

#### Scenario: Exact achievable total is disclosed

- **WHEN** the system proposes a geometry
- **THEN** it reports the exact total time as `cols × rows × unit_minutes`, which may differ from the administrator's requested target

#### Scenario: Administrator adjusts column count

- **WHEN** an administrator changes the proposed column count
- **THEN** the system recomputes rows, tile count, and exact total time, and shows the updated values before creation

### Requirement: Unique unguessable slug

The system SHALL assign each screen a randomly generated slug that is unique and usable as a URL path segment.

#### Scenario: Slug generated at creation

- **WHEN** a screen is created
- **THEN** the system assigns a random slug that does not collide with any existing screen's slug

#### Scenario: Screen is addressable by slug

- **WHEN** a member navigates directly to the URL containing a screen's slug
- **THEN** the system serves that screen without requiring navigation through the screen list

#### Scenario: Slug is not administrator-chosen

- **WHEN** an administrator creates a screen
- **THEN** the system does not accept a caller-supplied slug

### Requirement: Membership controls access

The system SHALL restrict every screen to its explicit member list, except that administrators SHALL have access to every screen whether or not they are members.

Administrators are exempt because they must be able to manage a screen they have not joined. Without the exemption an administrator could not see a screen in order to add themselves to it, and a screen whose last administrator member was removed would become unmanageable.

#### Scenario: Member opens a screen

- **WHEN** a member requests a screen by slug
- **THEN** the system serves the screen

#### Scenario: Non-member requests a screen

- **WHEN** an authenticated non-administrator who is not a member requests a screen by slug
- **THEN** the system responds 404, not 403, so the existence of the screen is not disclosed

#### Scenario: Administrator opens a screen they do not belong to

- **WHEN** an administrator requests a screen by slug and is not a member of it
- **THEN** the system serves the screen

#### Scenario: Unauthenticated request for a screen

- **WHEN** an unauthenticated visitor requests a screen by slug
- **THEN** the system responds 401 and discloses nothing about whether the slug exists

### Requirement: Administrator screen directory

The system SHALL provide administrators a listing of every screen, separate from the member-scoped personal listing, so that screens they do not belong to can be found and managed.

#### Scenario: Administrator lists every screen

- **WHEN** an administrator opens the screen directory
- **THEN** the system lists every screen regardless of membership, each with its title, progress, and archive status

#### Scenario: Non-administrator requests the directory

- **WHEN** a non-administrator requests the screen directory
- **THEN** the system rejects the request with 403

#### Scenario: Directory distinguishes the administrator's own screens

- **WHEN** an administrator views the directory
- **THEN** each entry indicates whether the administrator is a member of that screen

### Requirement: Administrator manages membership

The system SHALL allow administrators to add and remove members on a screen.

#### Scenario: Administrator adds a member

- **WHEN** an administrator adds an existing user to a screen
- **THEN** that user gains access to the screen and it appears in their screen list

#### Scenario: Non-administrator attempts to add a member

- **WHEN** a non-administrator attempts to modify a screen's membership
- **THEN** the system rejects the request with 403

### Requirement: Member removal offers a choice about their entries

The system SHALL require an administrator removing a member to choose whether that member's progress entries are retained or deleted, and SHALL disclose the resulting change to the screen total before confirmation.

#### Scenario: Removal retaining entries

- **WHEN** an administrator removes a member and chooses to retain their entries
- **THEN** the member loses access, their entries remain counted in the screen total, and the revealed tile count is unchanged

#### Scenario: Removal deleting entries

- **WHEN** an administrator removes a member and chooses to delete their entries
- **THEN** the member loses access, their entries are soft-deleted, and the screen total and revealed tile count decrease accordingly

#### Scenario: Impact disclosed before confirmation

- **WHEN** an administrator opens the removal confirmation for a member who has logged time
- **THEN** the system shows how many minutes and how many revealed tiles would be removed by the delete option

### Requirement: Member-scoped screen list

The system SHALL provide each authenticated user a list of exactly the screens they are a member of, separating active from archived screens.

#### Scenario: User views their screen list

- **WHEN** an authenticated user opens their screen list
- **THEN** the system lists only screens where they are a member, and lists no others

#### Scenario: An administrator's personal list is still member-scoped

- **WHEN** an administrator opens their own screen list
- **THEN** the system lists only screens they are a member of, not every screen; the full set is reached through the administrator screen directory instead

#### Scenario: Archived screens are separated

- **WHEN** a user's screen list contains archived screens
- **THEN** the system presents them in a section distinct from active screens

### Requirement: Participant count is visible but member identities are not

The system SHALL display the number of participants on a screen to all its members, and SHALL disclose member identities only to administrators.

#### Scenario: Member sees the participant count

- **WHEN** a member views a screen
- **THEN** the system shows the number of participants

#### Scenario: Member cannot see who the participants are

- **WHEN** a non-administrator member views a screen or queries its membership
- **THEN** the system does not disclose the usernames of other members

#### Scenario: Administrator sees the member list

- **WHEN** an administrator views a screen's membership
- **THEN** the system lists each member by username

### Requirement: Administrator edits a screen

The system SHALL allow administrators to edit a screen's title, and to edit its `cols`, `rows`, and `unit_minutes` after entries exist.

#### Scenario: Title edited

- **WHEN** an administrator changes a screen's title to a non-empty value
- **THEN** the system saves it and the new title appears in the screen header and the screen list

#### Scenario: Geometry edit warns before confirmation

- **WHEN** an administrator changes `cols`, `rows`, or `unit_minutes` on a screen that has entries
- **THEN** the system shows the resulting tile count and revealed tile count before confirmation, warning that the picture may become less revealed and that the reveal order will change

#### Scenario: Logged time survives a geometry edit

- **WHEN** a geometry edit is confirmed
- **THEN** no entry is modified or deleted and the screen's total logged time is unchanged
