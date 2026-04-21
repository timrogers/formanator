//! Helpers shared between the `forma_api.rs`, `cli.rs` and `llm_api.rs`
//! integration test binaries.
//!
//! Each integration test binary in `tests/` is compiled as its own crate, so
//! we include this file with `#[path = "common/mod.rs"] mod common;` rather
//! than relying on Rust's normal module resolution. Not every binary uses
//! every helper here, so individual items are annotated `#[allow(dead_code)]`.

#![allow(dead_code)]

use std::path::PathBuf;

/// JWT-shaped auth token returned by `magic_link_exchange_response.json`. The
/// integration tests assert that the login flow round-trips this exact value
/// from the fixture into the on-disk config.
pub const FIXTURE_AUTH_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImZvcm1hLXRlc3Qta2V5In0.eyJ1c2VyX2lkIjoiMTExMTExMTEtMTExMS00MTExLTgxMTEtMTExMTExMTExMTExIiwiY29tcGFueV9pZCI6Imdsb2JleF9pbmR1c3RyaWVzIiwiZW1haWwiOiJhbGV4LmRvZUBnbG9iZXguZXhhbXBsZSIsImlhdCI6MTc0NTIyMjgzNiwiZXhwIjoxNzQ1MzA5MjM2LCJpc3MiOiJmb3JtYS10ZXN0IiwicGFkZGluZyI6Inh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4eHh4In0.YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE";

/// Read a file from `tests/fixtures/` by name, panicking with a helpful
/// message on I/O errors.
pub fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
}

/// Create a temporary file with a `.jpg` extension and a minimal valid-looking
/// JPEG byte sequence. Suitable for tests that need to attach a receipt to a
/// multipart Forma claim request without depending on real image data.
pub fn make_fake_receipt() -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut f = tempfile::Builder::new()
        .suffix(".jpg")
        .tempfile()
        .expect("tempfile");
    // SOI + JFIF marker + EOI is enough to satisfy reqwest's multipart
    // streaming and to look approximately like a JPEG to anything that peeks.
    f.write_all(&[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0xFF, 0xD9,
    ])
    .expect("write fake receipt");
    f
}
