## ADDED Requirements

### Requirement: The installation has an administrator-editable name

The system SHALL store one name for the whole installation, SHALL allow administrators and only administrators to change it, and SHALL show it wherever the application names itself.

#### Scenario: Administrator renames the installation

- **WHEN** an administrator submits a new name
- **THEN** the system stores it and every page that names the application shows the new name, including the header, the sign-in page, the invitation page, and the browser tab title

#### Scenario: Name is trimmed

- **WHEN** a name arrives with leading or trailing whitespace
- **THEN** the system stores the trimmed name and reports back what it stored

#### Scenario: A blank name is refused

- **WHEN** an administrator submits an empty or whitespace-only name
- **THEN** the system rejects the request and the stored name is unchanged

#### Scenario: An overlong name is refused

- **WHEN** an administrator submits a name longer than 60 characters
- **THEN** the system rejects the request, reports the limit, and the stored name is unchanged

#### Scenario: A non-administrator cannot rename it

- **WHEN** an authenticated non-administrator attempts to change the name
- **THEN** the system rejects the request with 403 and stores nothing

#### Scenario: An anonymous caller cannot rename it

- **WHEN** someone with no session attempts to change the name
- **THEN** the system rejects the request with 401 and stores nothing

### Requirement: The name is readable without a session

The system SHALL disclose the installation's name to callers who are not authenticated, because the sign-in and invitation pages display it before anyone can have a session.

#### Scenario: A visitor reads the name before signing in

- **WHEN** someone with no session requests the site settings
- **THEN** the system returns the name

#### Scenario: A rename is not served stale

- **WHEN** the system returns the name
- **THEN** it instructs caches not to store the response, so a rename is visible on the next load

### Requirement: An installation always has a name

The system SHALL ensure a name exists at all times, so that no page has to render a nameless state.

#### Scenario: A fresh installation

- **WHEN** the database is created
- **THEN** the name is already set to the built-in default, and reading it requires no fallback

#### Scenario: The name cannot be read

- **WHEN** the client cannot reach the API to read the name
- **THEN** it renders the built-in default rather than leaving the name blank
