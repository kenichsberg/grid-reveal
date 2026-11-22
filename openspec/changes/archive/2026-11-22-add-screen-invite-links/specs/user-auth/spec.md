## MODIFIED Requirements

### Requirement: Administrator creates user accounts

The system SHALL allow administrators, and only administrators, to create user
accounts with a username and password. An account MAY also be created by the
holder of an invite, who chooses their own credentials. Open self-service
registration SHALL NOT exist: without either an administrator or a valid invite
token, there is no way to create an account.

#### Scenario: Administrator creates an account

- **WHEN** an administrator submits a username and password for a new account
- **THEN** the system creates the account, stores the password as an argon2id hash, and never stores or returns the plaintext password

#### Scenario: Non-administrator attempts account creation

- **WHEN** an authenticated non-administrator calls the account creation endpoint
- **THEN** the system rejects the request with 403 and creates no account

#### Scenario: Administrator designates another administrator

- **WHEN** an administrator creates an account with the administrator flag set
- **THEN** the system creates the account with `is_admin = true`

#### Scenario: Account created through an invite

- **WHEN** the holder of an open invite submits a username and password
- **THEN** the system creates the account with those credentials, and the administrator who issued the invite never learns the password

#### Scenario: No account without an administrator or an invite

- **WHEN** an unauthenticated visitor attempts to create an account without an invite token
- **THEN** the system refuses, since there is no endpoint that accepts one
