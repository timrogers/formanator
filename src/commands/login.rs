use anyhow::{Context, Result, bail};
use colored::Colorize;
use url::Url;

use crate::cli::LoginArgs;
use crate::config::{Config, read_config, store_config};
use crate::forma::exchange_id_and_tk_for_access_token;
use crate::prompt::prompt;
use crate::verbose;

const FORMA_LOGIN_URL: &str = "https://client.joinforma.com/login?type=magic";
const MAX_LINK_HOPS: usize = 5;

/// Resolve an emailed magic link, returning its `id`/`tk`.
///
/// Tracking URLs require a network request; direct links are parsed locally.
pub fn parse_emailed_forma_magic_link(input: &str) -> Result<(String, String)> {
    resolve_magic_link(input, fetch_tracking_redirect)
}

fn required_query(url: &Url, name: &str) -> Result<String> {
    let mut values = url.query_pairs().filter(|(key, _)| key == name);
    match (values.next(), values.next()) {
        (Some((_, value)), None) if !value.is_empty() => Ok(value.into_owned()),
        _ => bail!("Forma magic links require exactly one nonempty `{name}` query parameter."),
    }
}

fn resolve_magic_link(
    input: &str,
    mut redirect: impl FnMut(&Url) -> Result<String>,
) -> Result<(String, String)> {
    let mut link = Url::parse(input.trim()).context("Could not parse the input as a URL.")?;
    for hop in 0..=MAX_LINK_HOPS {
        if link.scheme() != "https"
            || !link.username().is_empty()
            || link.password().is_some()
            || link.port().is_some()
            || link.fragment().is_some()
        {
            bail!(
                "Forma magic links require HTTPS without user information, custom ports, or fragments."
            );
        }

        if matches!(
            (link.host_str(), link.path()),
            (Some("client.joinforma.com"), "/auth/magic")
                | (Some("api.joinforma.com"), "/client/auth/v2/login/magic")
        ) {
            return Ok((required_query(&link, "id")?, required_query(&link, "tk")?));
        }

        let legacy = link.host_str() == Some("joinforma.page.link") && link.path() == "/";
        let tracking = link.path() == "/ls/click"
            && link
                .host_str()
                .and_then(|host| host.strip_prefix("url"))
                .and_then(|host| host.strip_suffix(".joinforma.com"))
                .is_some_and(|number| {
                    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
                });
        if !legacy && !tracking {
            bail!("Unsupported Forma magic-link host or path.");
        }
        if hop == MAX_LINK_HOPS {
            bail!("Forma magic link exceeded the redirect limit.");
        }
        if legacy {
            link = Url::parse(&required_query(&link, "link")?)
                .context("The `link` query parameter is not a valid URL.")?;
        } else {
            required_query(&link, "upn")?;
            let location = redirect(&link)?;
            link = link
                .join(&location)
                .context("Invalid Forma redirect URL.")?;
        }
    }
    unreachable!("the final hop either returns credentials or rejects the link")
}

fn fetch_tracking_redirect(link: &Url) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to build Forma tracking client.")?;
    // Do not retain reqwest errors: they can include the credential-bearing URL.
    let response = client
        .get(link.clone())
        .send()
        .map_err(|_| anyhow::anyhow!("Failed to resolve Forma tracking link."))?;
    if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
        bail!(
            "Forma tracking link did not return a redirect (HTTP {}).",
            response.status().as_u16()
        );
    }
    response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .context("Forma tracking redirect has a missing or invalid Location header.")
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
    // Preserve the auto-update check timestamp if a previous config exists, so
    // logging in doesn't immediately re-trigger the daily update check.
    let last_update_check_timestamp = read_config()
        .ok()
        .flatten()
        .and_then(|c| c.last_update_check_timestamp);
    store_config(&Config {
        access_token,
        email: None,
        last_update_check_timestamp,
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
    fn parses_a_current_emailed_magic_link() {
        let inner = "https://client.joinforma.com/auth/magic?id=abc123&tk=xyz789";
        let encoded = url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>();
        let outer = format!("https://joinforma.page.link/?link={encoded}");

        let (id, tk) = parse_emailed_forma_magic_link(&outer).expect("should parse");
        assert_eq!(id, "abc123");
        assert_eq!(tk, "xyz789");
    }

    #[test]
    fn parses_a_decoded_current_magic_link() {
        let link = "https://client.joinforma.com/auth/magic?id=abc123&tk=xyz789";

        let (id, tk) = parse_emailed_forma_magic_link(link).expect("should parse");
        assert_eq!(id, "abc123");
        assert_eq!(tk, "xyz789");
    }

    #[test]
    fn rejects_links_with_the_wrong_host() {
        let result = parse_emailed_forma_magic_link("https://evil.example.com/?link=foo");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_wrappers_with_an_untrusted_inner_host() {
        let inner = "https://evil.example.com/auth/magic?id=abc123&tk=xyz789";
        let encoded = url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>();
        let outer = format!("https://joinforma.page.link/?link={encoded}");

        let result = parse_emailed_forma_magic_link(&outer);
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
        assert!(format!("{err}").contains("tk"));
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

    #[test]
    fn resolves_tracking_redirect_without_visiting_the_magic_target() {
        let server = httpmock::MockServer::start();
        let destination = server.url("/must-not-be-visited");
        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/ls/click")
                .query_param("upn", "synthetic-tracking-token");
            then.status(302).header("Location", &destination);
        });
        let target = server.mock(|when, then| {
            when.path("/must-not-be-visited");
            then.status(200);
        });
        let result = resolve_magic_link(
            "https://url123.joinforma.com/ls/click?upn=synthetic-tracking-token",
            |url| {
                assert_eq!(url.host_str(), Some("url123.joinforma.com"));
                let mut local = Url::parse(&server.url("/ls/click")).unwrap();
                local.set_query(url.query());
                assert_eq!(fetch_tracking_redirect(&local)?, destination);
                Ok(
                    "https://client.joinforma.com/auth/magic?id=synthetic-id&tk=synthetic-token"
                        .into(),
                )
            },
        )
        .unwrap();
        assert_eq!(result, ("synthetic-id".into(), "synthetic-token".into()));
        mock.assert_calls(1);
        target.assert_calls(0);
    }

    #[test]
    fn resolves_tracking_through_legacy_wrapper_and_tracking_hops() {
        let direct = "https://api.joinforma.com/client/auth/v2/login/magic?id=synthetic-id&tk=synthetic-token";
        let wrapper =
            Url::parse_with_params("https://joinforma.page.link/", &[("link", direct)]).unwrap();
        let mut calls = 0;
        let result = resolve_magic_link(
            "https://url1.joinforma.com/ls/click?upn=synthetic-tracking-token",
            |_| {
                calls += 1;
                Ok(if calls == 1 {
                    "https://url2.joinforma.com/ls/click?upn=synthetic-tracking-token".into()
                } else {
                    wrapper.to_string()
                })
            },
        )
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(result, ("synthetic-id".into(), "synthetic-token".into()));
    }

    #[test]
    fn rejects_unsafe_or_malformed_links_before_network_access() {
        for input in [
            "https://url.joinforma.com/ls/click?upn=synthetic",
            "https://urlx.joinforma.com/ls/click?upn=synthetic",
            "https://url1.joinforma.com.evil.example/ls/click?upn=synthetic",
            "https://url1.joinforma.com/other?upn=synthetic",
            "http://url1.joinforma.com/ls/click?upn=synthetic",
            "https://synthetic@url1.joinforma.com/ls/click?upn=synthetic",
            "https://url1.joinforma.com:8443/ls/click?upn=synthetic",
            "https://url1.joinforma.com/ls/click?upn=synthetic#fragment",
            "https://url1.joinforma.com/ls/click",
            "https://url1.joinforma.com/ls/click?upn=",
            "https://url1.joinforma.com/ls/click?upn=synthetic&upn=duplicate",
            "https://client.joinforma.com/auth/magic?id=&tk=synthetic",
            "https://client.joinforma.com/auth/magic?id=synthetic&tk=",
            "https://client.joinforma.com/auth/magic?id=synthetic&id=duplicate&tk=synthetic",
            "https://client.joinforma.com/auth/magic?id=synthetic&tk=synthetic&tk=duplicate",
            "https://joinforma.page.link/?link=x&link=y",
        ] {
            assert!(resolve_magic_link(input, |_| panic!("unexpected network request")).is_err());
        }
    }

    #[test]
    fn rejects_unsafe_redirect_targets_without_requesting_them() {
        for destination in [
            "https://evil.example/?tk=synthetic-secret",
            "http://client.joinforma.com/auth/magic?id=synthetic&tk=synthetic-secret",
            "https://api.joinforma.com/other?tk=synthetic-secret",
            "https://127.0.0.1/ls/click?upn=synthetic-secret",
            "https://client.joinforma.com:8443/auth/magic?id=synthetic&tk=synthetic-secret",
            "https://synthetic-secret@client.joinforma.com/auth/magic?id=synthetic&tk=synthetic",
            "https://[invalid?tk=synthetic-secret",
        ] {
            let mut calls = 0;
            let error = resolve_magic_link(
                "https://url1.joinforma.com/ls/click?upn=synthetic-secret",
                |_| {
                    calls += 1;
                    Ok(destination.into())
                },
            )
            .unwrap_err();
            assert_eq!(calls, 1);
            assert!(!format!("{error:#}").contains("synthetic-secret"));
        }
    }

    #[test]
    fn bounds_tracking_redirect_loops() {
        let input = "https://url1.joinforma.com/ls/click?upn=synthetic";
        let mut calls = 0;
        let error = resolve_magic_link(input, |_| {
            calls += 1;
            Ok("/ls/click?upn=synthetic".into())
        })
        .unwrap_err();
        assert_eq!(calls, MAX_LINK_HOPS);
        assert!(error.to_string().contains("redirect limit"));
    }

    #[test]
    fn rejects_non_redirect_responses_and_missing_locations() {
        for status in [200, 302, 400, 500] {
            let server = httpmock::MockServer::start();
            let mock = server.mock(|when, then| {
                when.method(httpmock::Method::GET).path("/ls/click");
                then.status(status).body("synthetic-secret");
            });
            let url = Url::parse(&server.url("/ls/click?upn=synthetic-secret")).unwrap();
            let error = fetch_tracking_redirect(&url).unwrap_err();
            assert!(!format!("{error:#}").contains("synthetic-secret"));
            mock.assert_calls(1);
        }
    }
}
