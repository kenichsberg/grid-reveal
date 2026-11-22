## ADDED Requirements

### Requirement: Administrator generates an invite link

The system SHALL allow administrators, and only administrators, to generate an
invite for a specific screen. The invite SHALL carry a random, unguessable
token and SHALL grant membership of that one screen and nothing else. An
administrator MAY attach a short label to an invite to record who it was
intended for.

#### Scenario: Invite generated

- **WHEN** an administrator generates an invite for a screen
- **THEN** the system stores it against that screen with a newly generated token, and returns the token so a link can be built from it

#### Scenario: Non-administrator attempts to generate an invite

- **WHEN** an authenticated non-administrator requests an invite for a screen
- **THEN** the system rejects the request and stores nothing

#### Scenario: Each invite is distinct

- **WHEN** an administrator generates two invites for the same screen
- **THEN** the system issues two different tokens, each usable once

### Requirement: Invite can be inspected without an account

The system SHALL allow anyone holding an invite token to see what the invite is
for — the screen's title and picture — and whether it is still open, without
signing in and without consuming the invite.

#### Scenario: Holder opens the link

- **WHEN** an unauthenticated visitor presents a valid, open invite token
- **THEN** the system returns the screen's title and picture and reports the invite as open

#### Scenario: Inspecting does not consume

- **WHEN** an invite is inspected any number of times
- **THEN** it remains open, so that a prefetch, a link preview, or a reloaded page does not spend it

#### Scenario: Unknown token

- **WHEN** a visitor presents a token that does not exist
- **THEN** the system responds 404 and discloses nothing about any screen

#### Scenario: Closed invite is reported, not hidden

- **WHEN** a visitor presents a token that has been accepted, revoked, or whose screen's period has ended
- **THEN** the system reports that the invite is no longer open, so the page can say so rather than appearing broken

### Requirement: Accepting an invite creates an account and joins the screen

The system SHALL allow the holder of an open invite to create an account with a
username and password of their own choosing, and SHALL join that account to the
invite's screen and sign it in. The account creation, the membership, the
session, and the consumption of the invite SHALL all succeed together or not at
all.

#### Scenario: Invite accepted

- **WHEN** the holder of an open invite submits a valid username and password
- **THEN** the system creates the account, adds it to the screen, establishes a session, marks the invite accepted, and records which account accepted it

#### Scenario: Username already taken

- **WHEN** the holder submits a username that already exists
- **THEN** the system rejects the request, creates no account, and leaves the invite open so another username can be tried

#### Scenario: Password fails its constraints

- **WHEN** the holder submits a password that does not meet the length requirement
- **THEN** the system rejects the request and leaves the invite open

#### Scenario: Accepting while signed in

- **WHEN** the holder of an open invite presents a valid session
- **THEN** the system joins that existing account to the screen and consumes the invite, rather than creating a second account

#### Scenario: Invited member joins as an ordinary member

- **WHEN** an invite is accepted
- **THEN** the resulting account is an ordinary member, never an administrator

### Requirement: An invite may be used once

The system SHALL permit at most one acceptance per invite, decided by the
database rather than by a read followed by a write.

#### Scenario: Second acceptance refused

- **WHEN** an invite that has already been accepted is presented again
- **THEN** the system refuses it, creates no account, and reports that the invite is no longer open

#### Scenario: Two acceptances at once

- **WHEN** two acceptances of the same invite are submitted simultaneously
- **THEN** exactly one succeeds and the other is refused, with no second account created

### Requirement: An invite lives exactly as long as its screen's period

The system SHALL treat an invite as open only while its screen's phase, as
resolved for the caller's own calendar day, is upcoming or active. The system
SHALL NOT store a separate expiry date for an invite.

#### Scenario: Invite for a screen that has not started

- **WHEN** an invite is presented for a screen whose start date has not arrived
- **THEN** the invite is open, since invitations go out before a period begins

#### Scenario: Invite for a screen whose period has ended

- **WHEN** an invite is presented for a screen whose period has ended
- **THEN** the invite is closed, and no account is created

#### Scenario: Invite for an archived screen

- **WHEN** an invite is presented for a screen that has been archived
- **THEN** the invite is closed

#### Scenario: Extending the period reopens its invites

- **WHEN** an administrator extends the end date of a screen whose period had ended
- **THEN** invites for that screen that were never accepted or revoked are open again, because the invite's life was never anything but the screen's

### Requirement: Administrator revokes and reviews invites

The system SHALL allow administrators to withdraw an invite that has not been
accepted, and to see a screen's invites with their state.

#### Scenario: Invite revoked

- **WHEN** an administrator revokes a pending invite
- **THEN** the invite is closed and presenting its token creates no account

#### Scenario: Revoking an accepted invite changes nothing

- **WHEN** an administrator revokes an invite that was already accepted
- **THEN** the member who accepted it keeps their account and their membership

#### Scenario: Administrator reviews a screen's invites

- **WHEN** an administrator opens a screen's management page
- **THEN** the system reports each invite's label, when it was created, whether it is open, and which account accepted it if any
