## MODIFIED Requirements

### Requirement: Membership controls access

The system SHALL restrict every screen to its explicit member list, except that administrators SHALL have access to every screen whether or not they are members. Membership SHALL be acquired either by an administrator adding an account or by that account accepting an invite to the screen.

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

#### Scenario: Membership acquired by invitation

- **WHEN** an account accepts an invite to a screen
- **THEN** it is a member of that screen and of no other, and the screen appears in its screen list
