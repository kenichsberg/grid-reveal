## 1. Resolve the caller's day

- [x] 1.1 Add a `ClientToday` extractor beside `AuthUser` in `apps/api/src/auth/extract.rs`, with the header name as a public constant so the CORS layer and the tests share one spelling
- [x] 1.2 Implement resolution: parse the header as a plain `NaiveDate`, accept it when it is within one day of `Utc::now().date_naive()`, otherwise fall back to the UTC date and log the discarded value at debug level
- [x] 1.3 Make the extractor infallible — it resolves a date for every request and never rejects one
- [x] 1.4 Unit-test resolution against a fixed server date: honoured, a day behind, a day ahead, two days out, malformed, and absent

## 2. Thread the resolved day through the handlers

- [x] 2.1 Change `resolve_date` in `apps/api/src/routes/entries.rs` to take the resolved day as an argument instead of reading the clock, and unit-test it directly
- [x] 2.2 Replace the four `Utc::now().date_naive()` call sites in `entries.rs` with the extractor
- [x] 2.3 Replace the two `ensure_writable` call sites in `apps/api/src/routes/goals.rs` with the extractor
- [x] 2.4 Replace the three phase-deriving call sites in `apps/api/src/routes/screens.rs` (list, admin directory, detail) with the extractor
- [x] 2.5 Confirm the nine production call sites are gone: `rg 'Utc::now\(\)\.date_naive\(\)' apps/api/src` matches only the extractor and `#[cfg(test)]` modules
- [x] 2.6 Leave `apps/api/src/access.rs` and `Screen::phase` untouched — they already take the day as a parameter

## 3. Let the header reach the server

- [x] 3.1 Add the header to `allow_headers` in `build_cors` in `apps/api/src/routes/mod.rs`, without which every cross-origin request fails preflight
- [x] 3.2 Send the header on every request from `apps/web/src/lib/api/client.ts`, using the existing local-date helper in `$lib/format.ts`
- [x] 3.3 Confirm no DTO changes and no `scripts/gen-types.sh` run are needed, since the contract is a header rather than a field

## 4. Prove the boundaries

- [x] 4.1 Contract test: an entry dated the caller's day is accepted when the server's UTC day is the day before
- [x] 4.2 Contract test: a screen whose start date is the caller's day accepts entries and goals while the server's UTC day is still the day before
- [x] 4.3 Contract test: a screen whose end date is the caller's day accepts entries while the server's UTC day is already the day after
- [x] 4.4 Contract test: a date beyond the bound is discarded, and a write to a long-closed screen is still refused
- [x] 4.5 Contract test: a request with no header behaves exactly as before the change
- [x] 4.6 Run `cargo test` and `npm run check`, and verify the full suite passes
- [x] 4.7 Give CI a PostgreSQL service so the boundary tests run there, and correct the workflow comment that said the suite needs no database
- [x] 4.8 Contract test the read path too: the phase a screen reports is the caller's own, and the label a member is shown agrees with whether they may write

## 5. Finish

- [x] 5.1 Document the header where the API contract is described, including that it is optional and that omitting it yields UTC semantics
- [x] 5.2 Confirm the Caddy configuration in `deploy/` forwards the header unchanged
- [x] 5.3 Record the deploy order — API first, web second — in the change's notes, since the API must accept the header before anything sends it
