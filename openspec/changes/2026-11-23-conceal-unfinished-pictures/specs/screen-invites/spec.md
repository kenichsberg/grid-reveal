## MODIFIED Requirements

### Requirement: Invite can be inspected without an account

The system SHALL allow anyone holding an invite token to see what the invite is
for — the screen's title, how far its board has been uncovered, and how many
people are on it — and whether it is still open, without signing in and without
consuming the invite. It SHALL NOT disclose the screen's background picture.

#### Scenario: Holder opens the link

- **WHEN** an unauthenticated visitor presents a valid, open invite token
- **THEN** the system returns the screen's title, its revealed and total tile counts, and its participant count, and reports the invite as open

#### Scenario: The picture is not disclosed to an invitee

- **WHEN** an invite is inspected, whatever the state of the board
- **THEN** the response carries no picture URL: a link may be forwarded to anyone, and the picture is the thing the group is uncovering

#### Scenario: The standing reported is the screen's own

- **WHEN** someone joins through an invitation and a link to that screen is inspected again
- **THEN** the reported participant count has grown by one

#### Scenario: Inspecting does not consume

- **WHEN** an invite is inspected any number of times
- **THEN** it remains open, so that a prefetch, a link preview, or a reloaded page does not spend it

#### Scenario: Unknown token

- **WHEN** a visitor presents a token that does not exist
- **THEN** the system responds 404 and discloses nothing about any screen

#### Scenario: Closed invite is reported, not hidden

- **WHEN** a visitor presents a token that has been accepted, revoked, or whose screen's period has ended
- **THEN** the system reports that the invite is no longer open, so the page can say so rather than appearing broken
