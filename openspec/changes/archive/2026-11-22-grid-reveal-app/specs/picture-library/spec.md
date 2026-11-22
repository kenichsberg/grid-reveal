## ADDED Requirements

### Requirement: Administrator uploads a picture

The system SHALL allow administrators, and only administrators, to upload background pictures.

#### Scenario: Valid upload

- **WHEN** an administrator uploads a JPEG, PNG, or WebP image of 10 MB or less
- **THEN** the system accepts it, processes it, stores it in object storage, and records a library entry with its storage key and final pixel dimensions

#### Scenario: Unsupported format

- **WHEN** an administrator uploads a file that is not JPEG, PNG, or WebP
- **THEN** the system rejects the upload and reports the accepted formats

#### Scenario: File exceeds the size limit

- **WHEN** an administrator uploads a file larger than 10 MB
- **THEN** the system rejects the upload and reports the size limit

#### Scenario: Non-administrator attempts upload

- **WHEN** an authenticated non-administrator attempts to upload a picture
- **THEN** the system rejects the request with 403 and stores nothing

### Requirement: Server-side image processing

The system SHALL downscale uploaded images so that neither dimension exceeds 2560 pixels, re-encode them to WebP, and record the resulting dimensions.

#### Scenario: Oversized image is downscaled

- **WHEN** an uploaded image has a dimension larger than 2560 pixels
- **THEN** the system scales it down preserving aspect ratio so the larger dimension is 2560 pixels, and stores the result

#### Scenario: Small image is not upscaled

- **WHEN** an uploaded image has both dimensions at or below 2560 pixels
- **THEN** the system preserves its dimensions and does not enlarge it

#### Scenario: Dimensions are recorded for geometry derivation

- **WHEN** processing completes
- **THEN** the system stores the final width and height on the library entry so screen geometry can be derived from the aspect ratio

### Requirement: Shared library across administrators

The system SHALL expose the picture library to all administrators regardless of who uploaded each picture.

#### Scenario: Administrator browses pictures uploaded by another administrator

- **WHEN** an administrator opens the picture library
- **THEN** the system lists every picture that is not soft-deleted, including those uploaded by other administrators

#### Scenario: Non-administrator cannot browse the library

- **WHEN** a non-administrator requests the picture library
- **THEN** the system rejects the request with 403

### Requirement: Soft-delete removes a picture from selection

The system SHALL allow an administrator to soft-delete a picture, which hides it from selection for new screens while leaving screens that already use it unaffected.

#### Scenario: Soft-deleted picture disappears from the picker

- **WHEN** an administrator soft-deletes a picture and then opens the picture library
- **THEN** the system omits that picture from the list and it cannot be chosen for a new screen

#### Scenario: Existing screen keeps rendering a soft-deleted picture

- **WHEN** a screen references a picture that has since been soft-deleted
- **THEN** that screen continues to display the picture normally

### Requirement: Hard-delete is blocked while referenced

The system SHALL permanently delete a picture and its stored object only when no screen references it, and SHALL reject the deletion otherwise.

#### Scenario: Hard-delete of an unreferenced picture

- **WHEN** an administrator hard-deletes a picture that no screen references
- **THEN** the system removes the object from storage and deletes the library row

#### Scenario: Hard-delete of a referenced picture

- **WHEN** an administrator attempts to hard-delete a picture that at least one screen references
- **THEN** the system rejects the deletion, reports how many screens reference it, and removes nothing

#### Scenario: Non-administrator attempts deletion

- **WHEN** a non-administrator attempts to soft-delete or hard-delete a picture
- **THEN** the system rejects the request with 403
