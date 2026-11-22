## 1. Store an invite

- [x] 1.1 Add `apps/api/migrations/0011_screen_invites.sql`: `id`, `screen_id`, `token`, `label`, `created_by`, `created_at`, `accepted_at`, `accepted_by`, `revoked_at`
- [x] 1.2 Unique index on `token`; index on `screen_id` for the administrator's listing
- [x] 1.3 `ON DELETE CASCADE` from `screens` and from `users` on `created_by`, so deleting either leaves nothing dangling; `accepted_by` nullable and `ON DELETE SET NULL`, since removing a member should not erase the record that an invite was used
- [x] 1.4 Comment the table with the one fact that is not obvious from its columns: there is no `expires_at`, and why
- [x] 1.5 Add a `ScreenInvite` row type to `apps/api/src/models.rs` with an `is_open(screen_phase)` predicate, unit-tested against every phase and against accepted and revoked rows

## 2. Issue and withdraw

- [x] 2.1 Create `apps/api/src/routes/invites.rs` and register it in `routes/mod.rs`
- [x] 2.2 `POST /screens/{slug}/invites` for administrators: generate a 24-character token from the existing slug alphabet, retrying on collision the way `unique_slug` does
- [x] 2.3 Validate and bound the optional label, reusing the trimming and length pattern from `validate_label` in `screens.rs`
- [x] 2.4 `DELETE /screens/{slug}/invites/{id}` for administrators (scoped to the screen, so the access check is the same one every other handler makes, and `/invites/{token}` stays free for the public endpoints): set `revoked_at`, leaving an accepted invite untouched
- [x] 2.5 `GET /screens/{slug}/invites` for administrators: every invite with its state and, where accepted, the username that accepted it
- [x] 2.6 Confirm a non-administrator gets the same answer for a screen they cannot see as for one that does not exist

## 3. Let someone in

- [x] 3.1 `GET /invites/{token}`, public: the screen's title and picture plus whether the invite is open; 404 for a token that does not exist
- [x] 3.2 Confirm the peek endpoint writes nothing, so a prefetch or a reload cannot spend an invite
- [x] 3.3 `POST /invites/{token}/accept`, public: validate the username and password with the existing helpers in `users.rs` and `password.rs` rather than new ones
- [x] 3.4 Consume with a conditional `UPDATE ... WHERE accepted_at IS NULL AND revoked_at IS NULL RETURNING id`; zero rows means someone else won, and the transaction rolls back
- [x] 3.5 Create the account, insert the membership, consume the invite, and mint the session inside one transaction
- [x] 3.6 Return the session cookie exactly as `auth::login` does, so the invited person arrives signed in
- [x] 3.7 Accepting with a valid session joins that account instead of creating one, and takes no username or password
- [x] 3.8 Resolve the screen's phase from `ClientToday`, so an invite's last day is the caller's day

## 4. Wire types and the public page

- [x] 4.1 Add the invite DTOs to `apps/api/src/dto.rs` and run `scripts/gen-types.sh`
- [x] 4.2 Add the four calls to `apps/web/src/lib/api/client.ts`
- [x] 4.3 Replace `PUBLIC_ROUTES` in `apps/web/src/routes/+layout.svelte` with a predicate covering `/login` and `/join/`, and use it in both the redirect effect and the unauthorized handler
- [x] 4.4 Add `apps/web/src/routes/join/[token]/+page.svelte`: the screen being joined, a username and password form, and a plain explanation when the invite is closed
- [x] 4.5 On success, store the session and land on the screen's board
- [x] 4.6 When the visitor is already signed in, offer to join as that account rather than showing a credential form

## 5. The administrator's side

- [x] 5.1 Add an invites section to `apps/web/src/routes/admin/screens/[slug]/+page.svelte`: generate, with the optional label
- [x] 5.2 Show the full URL once generated, built from `location.origin`, with a copy control
- [x] 5.3 List outstanding invites with their label and creation date, and a revoke control
- [x] 5.4 Show accepted invites with the username that accepted, so the list is a record of how each member got in
- [x] 5.5 Say on the page when invites for this screen stop working — the screen's end date — so a dead link is never a surprise

## 6. Prove it

- [x] 6.1 Integration test in `apps/api/tests/screen_invites.rs`: the whole path — generate, peek, accept, land a member on the screen
- [x] 6.2 Test: a second acceptance is refused and creates no account
- [x] 6.3 Test: a taken username is refused and the invite stays open, so a typo does not cost the invitation
- [x] 6.4 Test: an invite for a screen whose period has ended is closed, and one for a screen that has not started is open
- [x] 6.5 Test: extending a closed screen's end date reopens its unused invites
- [x] 6.6 Test: a revoked invite is closed; revoking an accepted one leaves the member alone
- [x] 6.7 Test: cloning a screen with pending invites gives the clone none
- [x] 6.8 Test: peeking repeatedly leaves the invite open
- [x] 6.9 Test: an unknown token is a 404 from both public endpoints
- [x] 6.10 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `npm run check`

## 7. Finish

- [x] 7.1 Document in the README how someone joins, and that an invite dies with its screen's period
- [x] 7.2 Record the deploy order — API first, so no link can exist before the endpoint that opens it
- [x] 7.3 Walk the whole path in the running app: generate a link as an administrator, open it in a signed-out window, create an account, and land on the board
