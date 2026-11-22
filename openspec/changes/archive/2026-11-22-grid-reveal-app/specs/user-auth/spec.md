## ADDED Requirements

### Requirement: Bootstrap admin account

The system SHALL create an initial administrator account on startup from the `ADMIN_USERNAME` and `ADMIN_PASSWORD` environment variables when no administrator account exists.

#### Scenario: First boot with no accounts

- **WHEN** the API starts and the `users` table contains no account with `is_admin = true`
- **THEN** the system creates an administrator account using `ADMIN_USERNAME` and `ADMIN_PASSWORD`, storing the password as an argon2id hash

#### Scenario: Subsequent boot with an existing admin

- **WHEN** the API starts and at least one administrator account already exists
- **THEN** the system makes no change to any account, even if the environment variables differ from stored values

#### Scenario: Bootstrap variables missing

- **WHEN** the API starts, no administrator exists, and either environment variable is unset or empty
- **THEN** the system fails to start and reports that admin bootstrap configuration is required

### Requirement: Administrator creates user accounts

The system SHALL allow administrators, and only administrators, to create user accounts with a username and password. Self-service registration SHALL NOT exist.

#### Scenario: Administrator creates an account

- **WHEN** an administrator submits a username and password for a new account
- **THEN** the system creates the account, stores the password as an argon2id hash, and never stores or returns the plaintext password

#### Scenario: Non-administrator attempts account creation

- **WHEN** an authenticated non-administrator calls the account creation endpoint
- **THEN** the system rejects the request with 403 and creates no account

#### Scenario: Administrator designates another administrator

- **WHEN** an administrator creates an account with the administrator flag set
- **THEN** the system creates the account with `is_admin = true`

### Requirement: Username constraints

The system SHALL require usernames to be between 3 and 32 characters and SHALL enforce uniqueness case-insensitively.

#### Scenario: Username already taken in a different case

- **WHEN** an administrator attempts to create the account `Alice` and an account `alice` already exists
- **THEN** the system rejects the request and reports that the username is taken

#### Scenario: Username too short

- **WHEN** an administrator submits a username shorter than 3 characters
- **THEN** the system rejects the request and reports the length constraint

#### Scenario: Login is case-insensitive on username

- **WHEN** a user whose stored username is `alice` logs in as `Alice` with the correct password
- **THEN** the system authenticates them successfully

### Requirement: Password constraints and hashing

The system SHALL require passwords of at least 8 characters and SHALL store them only as argon2id hashes.

#### Scenario: Password too short

- **WHEN** a password shorter than 8 characters is submitted at account creation, password change, or administrator reset
- **THEN** the system rejects the request and reports the minimum length

#### Scenario: Hash is never exposed

- **WHEN** any API response returns user data
- **THEN** the response contains no password hash and no plaintext password

### Requirement: Email is optional and unused

The system SHALL store an optional email address per user and SHALL NOT send email of any kind.

#### Scenario: Account created without an email

- **WHEN** an administrator creates an account and supplies no email address
- **THEN** the system creates the account with a null email and the account functions normally

#### Scenario: No email is sent on any operation

- **WHEN** any account operation occurs, including creation, password change, or password reset
- **THEN** the system sends no email and requires no mail transport configuration

### Requirement: Login establishes a session

The system SHALL authenticate a username and password pair and establish a server-side session identified by an opaque random token delivered as a cookie.

#### Scenario: Successful login

- **WHEN** a user submits a valid username and password
- **THEN** the system creates a session row with an expiry and sets a cookie with the attributes `HttpOnly`, `Secure`, `SameSite=Lax`, and a `Domain` covering both the web and API subdomains

#### Scenario: Incorrect password

- **WHEN** a user submits a valid username with an incorrect password
- **THEN** the system rejects the login with an error that does not reveal whether the username exists, and creates no session

#### Scenario: Unknown username

- **WHEN** a user submits a username that does not exist
- **THEN** the system rejects the login with the same error and timing characteristics as an incorrect password

### Requirement: Session validation and expiry

The system SHALL validate the session token on every authenticated request and SHALL reject expired or unknown tokens.

#### Scenario: Request with a valid session

- **WHEN** a request arrives bearing a session cookie whose token exists and has not expired
- **THEN** the system resolves the request to that user

#### Scenario: Request with an expired session

- **WHEN** a request arrives bearing a session token whose expiry has passed
- **THEN** the system rejects the request with 401 and the client is returned to the login screen

#### Scenario: Request with no session

- **WHEN** a request to an authenticated endpoint arrives with no session cookie
- **THEN** the system rejects the request with 401

### Requirement: Logout

The system SHALL allow a user to end their session.

#### Scenario: User logs out

- **WHEN** an authenticated user logs out
- **THEN** the system deletes the session row and clears the cookie, and subsequent requests with that token are rejected with 401

### Requirement: User changes own password

The system SHALL allow an authenticated user to change their own password by supplying their current password.

#### Scenario: Successful password change

- **WHEN** an authenticated user submits their correct current password and a valid new password
- **THEN** the system replaces the stored hash and the user's other sessions are revoked

#### Scenario: Incorrect current password

- **WHEN** an authenticated user submits an incorrect current password
- **THEN** the system rejects the change and leaves the stored hash unmodified

### Requirement: Administrator resets a password

The system SHALL allow an administrator to set a new password for any account without knowing the existing password, and SHALL revoke that account's sessions on reset.

#### Scenario: Administrator resets another user's password

- **WHEN** an administrator sets a new password for a user
- **THEN** the system replaces the stored hash and deletes all sessions belonging to that user

#### Scenario: Non-administrator attempts a reset

- **WHEN** a non-administrator attempts to reset another user's password
- **THEN** the system rejects the request with 403

#### Scenario: No self-service recovery exists

- **WHEN** a user forgets their password
- **THEN** the system offers no recovery flow and the user must ask an administrator to reset it
