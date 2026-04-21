# Test fixtures

This directory contains JSON fixtures that mirror the request and response
shapes used by the Forma API client in `src/forma.rs`. They are loaded by the
unit tests in the source modules and the integration tests in
`tests/forma_api.rs`.

The shapes are derived from the fields actually consumed/produced by the code.
Fields that the upstream API may also return but that the client does not look
at are omitted, except where useful for round-trip testing.

| File | Used for |
| ---- | -------- |
| `profile_response.json` | `GET /client/api/v3/settings/profile` (multiple wallets, including ineligible + aliases) |
| `profile_response_empty.json` | Profile response with no eligible wallets |
| `claims_list_page0.json` | `GET /client/api/v2/claims?page=0` (full page; numeric `limit`) |
| `claims_list_page1.json` | `GET /client/api/v2/claims?page=1` (final, partial page; **stringly-typed `limit`** — Forma sometimes returns `limit` as a string, and the client must tolerate both) |
| `claims_list_in_progress.json` | Single-page response containing in-progress and completed claims |
| `magic_link_request_response.json` | `POST /client/auth/v2/login/magic` (request body + success response) |
| `magic_link_exchange_response.json` | `GET /client/auth/v2/login/magic` (exchange success response) |
| `create_claim_response_success.json` | `POST /client/api/v2/claims` success body |
| `create_claim_response_unsuccessful.json` | 201 response with `success: false` |
| `error_invalid_jwt.json` | API error body whose message contains `JWT token is invalid` |
| `error_generic.json` | API error body with a user-friendly `errors.message` |
| `error_unknown_shape.json` | API error body whose shape is unrecognised |
| `template.csv` | The CSV template the `generate-template-csv` command writes |
