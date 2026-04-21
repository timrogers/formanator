//! Small interactive helpers shared by command implementations.

use std::io::{self, BufRead, Write};

use anyhow::Result;

/// Prompt for input on stdout, then read a line from stdin.
pub fn prompt(message: &str) -> Result<String> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write!(handle, "{message}")?;
    handle.flush()?;
    drop(handle);

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    // Strip trailing newline.
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(line)
}

/// Prompt for a yes/no answer; treats `y`/`yes` (case-insensitive) as yes.
pub fn prompt_yes_no(message: &str) -> Result<bool> {
    let answer = prompt(message)?.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}
