# Caller Date

## Purpose

Which calendar day a request happens on. Dates here are plain calendar
dates with no time component, so something has to decide whose day is
meant: the caller's, within a bound, and the server's UTC day when the
caller says nothing usable. Every day-granularity decision elsewhere —
an entry's default date, a future date, a screen's phase — resolves
through this.

## Requirements

### Requirement: Caller supplies the calendar day

The system SHALL resolve the calendar day of a request from a date supplied by
the caller, and SHALL use that day for every decision of day granularity:
defaulting an entry's date, rejecting a future date, and resolving a screen's
phase. The supplied date SHALL be a plain calendar date with no time component
and no timezone conversion.

#### Scenario: Caller's day is honoured

- **WHEN** a request carries a calendar date within one day of the server's UTC date
- **THEN** every day-granularity decision in that request resolves against the supplied date

#### Scenario: No date supplied

- **WHEN** a request carries no calendar date
- **THEN** the system resolves the day from its own UTC clock and the request succeeds

#### Scenario: Malformed date ignored

- **WHEN** a request carries a value that is not a calendar date
- **THEN** the system resolves the day from its own UTC clock, the request succeeds, and the discarded value is logged at debug level

### Requirement: Supplied day is bounded to one day of UTC

The system SHALL reject a supplied date that differs from its own UTC date by
more than one day, falling back to the UTC date. The bound SHALL be one day in
either direction, which spans every real offset from UTC-12 to UTC+14.

#### Scenario: A day behind UTC

- **WHEN** a caller supplies the day before the server's UTC date
- **THEN** the system resolves the day as supplied

#### Scenario: A day ahead of UTC

- **WHEN** a caller supplies the day after the server's UTC date
- **THEN** the system resolves the day as supplied

#### Scenario: Beyond the bound

- **WHEN** a caller supplies a date more than one day from the server's UTC date
- **THEN** the system resolves the day from its own UTC clock, so that a wrong device clock cannot move a screen's deadline

#### Scenario: Deadline cannot be extended arbitrarily

- **WHEN** a member supplies a date well inside a screen's period, on a screen whose period closed a month ago
- **THEN** the date is discarded as beyond the bound, the screen resolves as ended, and the write is refused

