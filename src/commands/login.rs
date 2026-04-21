use anyhow::{Context, Result, bail};
use colored::Colorize;
use url::Url;

use crate::cli::LoginArgs;
use crate::config::{Config, store_config};
use crate::forma::exchange_id_and_tk_for_access_token;
use crate::prompt::prompt;
use crate::verbose;

const FORMA_LOGIN_URL: &str = "https://client.joinforma.com/login?type=magic";

/// Parse a magic link emailed to the user, returning the embedded `id`/`tk`.
pub fn parse_emailed_forma_magic_link(input: &str) -> Result<(String, String)> {
    let parsed = Url::parse(input.trim()).context("Could not parse the input as a URL.")?;

    if parsed.host_str() != Some("joinforma.page.link") {
        bail!("Forma magic links are expected to have the hostname `joinforma.page.link`.");
    }
    if parsed.scheme() != "https" {
        bail!("Forma magic links are expected to have the protocol `https:`.");
    }

    let embedded = parsed
        .query_pairs()
        .find(|(k, _)| k == "link")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| {
            anyhow::anyhow!("Forma magic links are expected to have a `link` query parameter.")
        })?;

    let real_link =
        Url::parse(&embedded).context("The `link` query parameter is not a valid URL.")?;

    let mut id = None;
    let mut tk = None;
    for (k, v) in real_link.query_pairs() {
        if k == "id" {
            id = Some(v.into_owned());
        } else if k == "tk" {
            tk = Some(v.into_owned());
        }
    }

    match (id, tk) {
        (Some(id), Some(tk)) => Ok((id, tk)),
        _ => bail!(
            "Forma magic links are expected to have a `link` query parameter containing a URL with `id` and `tk` query parameters embedded inside."
        ),
    }
}

fn prompt_for_emailed_magic_link() -> Result<(String, String)> {
    loop {
        println!("Copy and paste the magic link from your email, then press Enter.");
        let input = prompt("> ")?;
        match parse_emailed_forma_magic_link(&input) {
            Ok(pair) => return Ok(pair),
            Err(_) => println!(
                "{}",
                "That doesn't look like a valid magic link. Please try again.".yellow()
            ),
        }
    }
}

pub fn run(args: LoginArgs) -> Result<()> {
    verbose::set(args.verbose);

    let (id, tk) = if let Some(link) = args.magic_link.as_deref() {
        parse_emailed_forma_magic_link(link)?
    } else {
        println!(
            "{}",
            "To log in, you'll need to enter your email address on the Forma login page to request a magic link.".blue()
        );
        println!(
            "{}",
            "Once you receive the magic link in your email, come back here to paste it.\n".blue()
        );
        println!(
            "{}",
            "Press Enter to open your browser to the Forma login page...".yellow()
        );
        let _ = prompt("")?;

        if let Err(e) = open::that(FORMA_LOGIN_URL) {
            eprintln!(
                "Couldn't open your browser automatically ({e}). Please open this URL manually: {FORMA_LOGIN_URL}"
            );
        }

        prompt_for_emailed_magic_link()?
    };

    let access_token = exchange_id_and_tk_for_access_token(&id, &tk)?;
    store_config(&Config {
        access_token,
        email: None,
    })?;

    println!("{}", "You are now logged in! 🥳".green());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_emailed_magic_link() {
        // The Forma magic-link email points at a `joinforma.page.link` URL
        // whose `link` query parameter contains the real magic link.
        let inner = "https://api.joinforma.com/client/auth/v2/login/magic?id=abc123&tk=xyz789";
        let encoded = url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>();
        let outer = format!("https://joinforma.page.link/?link={encoded}");

        let (id, tk) = parse_emailed_forma_magic_link(&outer).expect("should parse");
        assert_eq!(id, "abc123");
        assert_eq!(tk, "xyz789");
    }

    #[test]
    fn rejects_links_with_the_wrong_host() {
        let result = parse_emailed_forma_magic_link("https://evil.example.com/?link=foo");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_links_without_a_link_query_parameter() {
        let result = parse_emailed_forma_magic_link("https://joinforma.page.link/");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_https_links() {
        // The host check precludes this in practice but the scheme check is
        // still important for defence in depth.
        let result = parse_emailed_forma_magic_link("http://joinforma.page.link/?link=x");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_links_whose_inner_url_is_missing_id_or_tk() {
        let inner = "https://api.joinforma.com/client/auth/v2/login/magic?id=only";
        let encoded = url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>();
        let outer = format!("https://joinforma.page.link/?link={encoded}");
        let err = parse_emailed_forma_magic_link(&outer).expect_err("should fail");
        assert!(format!("{err}").contains("id"));
    }

    #[test]
    fn rejects_links_whose_inner_url_is_garbage() {
        let outer = "https://joinforma.page.link/?link=not%20a%20url";
        let result = parse_emailed_forma_magic_link(outer);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_input_that_is_not_a_url() {
        let result = parse_emailed_forma_magic_link("definitely not a url");
        assert!(result.is_err());
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let inner = "https://api.joinforma.com/client/auth/v2/login/magic?id=abc&tk=xyz";
        let encoded = url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>();
        let outer = format!("  https://joinforma.page.link/?link={encoded}\n");
        let (id, tk) = parse_emailed_forma_magic_link(&outer).expect("should parse");
        assert_eq!(id, "abc");
        assert_eq!(tk, "xyz");
    }
}
