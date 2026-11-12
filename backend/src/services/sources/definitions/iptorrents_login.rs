//! Signing in to IPTorrents with a username and password.
//!
//! IPTorrents is cookie-authenticated: a signed-in browser holds `uid` and `pass` cookies, and until
//! now a person pasted those in by hand. This module does what the browser does: read the login
//! page, fill in its form, post it, and keep the session cookies from the answer.
//!
//! **What is inferred and unverified.** Nothing here has been run against the real site. The shape
//! of the flow (`login.php` showing a form that posts to `take_login.php`, a `username` and a
//! `password` field, `uid` and `pass` cookies on success, a CAPTCHA / Cloudflare challenge /
//! two-factor step on some accounts) comes from Jackett's and Prowlarr's definitions and public
//! reports. The form is therefore *read* rather than assumed: its action, method and hidden fields
//! are taken from the page, and the username and password fields are found by their type. The
//! fixtures in `backend/tests/fixtures/iptorrents/` are hand-written to that structure.
//!
//! The site may refuse an automated sign-in. Each way it can is detected and reported as a
//! [`LoginProblem`] that says to use the cookie method instead; a login is never retried by this
//! module (the caller enforces one attempt per hour so an account is not locked).
//!
//! Nothing here logs or returns the username, password or cookie values.

use scraper::{Html, Selector};

/// What stopped a sign-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginProblem {
    /// A CAPTCHA the program cannot solve.
    Captcha,
    /// A Cloudflare "checking your browser" page.
    Cloudflare,
    /// The account asks for a two-factor code.
    TwoFactor,
    /// The site said the username or password is wrong.
    BadCredentials(String),
    /// The site is rate limiting.
    RateLimited,
    /// The login page has no form this code can use.
    NoForm,
    /// The answer was neither a success nor a failure it knows.
    Unrecognised(String),
}

impl LoginProblem {
    /// What to tell a person. The three "blocked" kinds say to use the cookie method.
    pub fn message(&self) -> String {
        match self {
            Self::Captcha => "IPTorrents asked for a CAPTCHA, which Librarian cannot solve. \
                 Use the cookie sign-in method instead."
                .to_string(),
            Self::Cloudflare => {
                "IPTorrents is behind a Cloudflare check that Librarian cannot pass. \
                 Use the cookie sign-in method instead."
                    .to_string()
            }
            Self::TwoFactor => "This IPTorrents account uses two-factor authentication, which \
                 Librarian cannot complete. Use the cookie sign-in method instead."
                .to_string(),
            Self::BadCredentials(detail) => {
                if detail.is_empty() {
                    "IPTorrents did not accept the username and password.".to_string()
                } else {
                    format!("IPTorrents did not accept the username and password ({detail}).")
                }
            }
            Self::RateLimited => {
                "IPTorrents is rate limiting sign-ins. Librarian will not try again for an hour."
                    .to_string()
            }
            Self::NoForm => "The IPTorrents login page has no form Librarian can fill in. \
                 Use the cookie sign-in method instead."
                .to_string(),
            Self::Unrecognised(detail) => format!(
                "IPTorrents answered the sign-in in a way Librarian does not recognise ({detail}). \
                 Use the cookie sign-in method instead."
            ),
        }
    }

    /// Whether the site is blocking automated sign-in as such (so a person has to use a browser),
    /// as opposed to refusing these credentials.
    pub fn is_blocked(&self) -> bool {
        matches!(
            self,
            Self::Captcha
                | Self::Cloudflare
                | Self::TwoFactor
                | Self::NoForm
                | Self::Unrecognised(_)
        )
    }
}

impl std::fmt::Display for LoginProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for LoginProblem {}

/// The login page's form, ready to be filled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginForm {
    /// Where to post, resolved against the page address.
    pub action: String,
    pub username_field: String,
    pub password_field: String,
    /// Every other field the form carries (hidden tokens, the submit button) with its value.
    pub extra_fields: Vec<(String, String)>,
}

impl LoginForm {
    /// The fields to post, with the credentials filled in. Extra fields come first, as a browser
    /// would send them in document order closely enough for a login form.
    pub fn filled(&self, username: &str, password: &str) -> Vec<(String, String)> {
        let mut fields = self.extra_fields.clone();
        fields.push((self.username_field.clone(), username.to_string()));
        fields.push((self.password_field.clone(), password.to_string()));
        fields
    }
}

/// The page is a challenge or a step the program cannot take, if it is.
pub fn detect_block(html: &str) -> Option<LoginProblem> {
    let lower = html.to_ascii_lowercase();
    let title = lower
        .split("<title>")
        .nth(1)
        .and_then(|rest| rest.split("</title>").next())
        .unwrap_or("");
    if title.contains("just a moment")
        || title.contains("attention required")
        || lower.contains("cdn-cgi/challenge-platform")
        || lower.contains("cf-chl-")
        || lower.contains("challenge-error-text")
    {
        return Some(LoginProblem::Cloudflare);
    }
    if lower.contains("g-recaptcha")
        || lower.contains("h-captcha")
        || lower.contains("cf-turnstile")
        || lower.contains("recaptcha/api")
        || lower.contains("hcaptcha.com")
        || lower.contains("data-sitekey")
    {
        return Some(LoginProblem::Captcha);
    }
    if title.contains("two-factor")
        || title.contains("2fa")
        || lower.contains("authenticator app")
        || lower.contains("two-factor authentication")
        || lower.contains("name=\"otp\"")
        || lower.contains("name=\"totp\"")
    {
        return Some(LoginProblem::TwoFactor);
    }
    None
}

fn resolve_action(page_url: &str, action: Option<&str>) -> Option<String> {
    let base = reqwest::Url::parse(page_url).ok()?;
    let target = action
        .filter(|action| !action.trim().is_empty())
        .unwrap_or("");
    base.join(target).ok().map(|url| url.to_string())
}

/// Find the login form in a page: the form holding a password field. Its action is resolved
/// against `page_url`; the username field is the first text, email or unlabelled input before the
/// password field; every other input keeps the value the page gave it.
pub fn parse_login_form(html: &str, page_url: &str) -> Result<LoginForm, LoginProblem> {
    let document = Html::parse_document(html);
    let form_selector = Selector::parse("form").expect("form selector");
    let input_selector = Selector::parse("input").expect("input selector");
    for form in document.select(&form_selector) {
        let inputs: Vec<_> = form.select(&input_selector).collect();
        let Some(password) = inputs.iter().find(|input| {
            input
                .value()
                .attr("type")
                .is_some_and(|kind| kind.eq_ignore_ascii_case("password"))
        }) else {
            continue;
        };
        let Some(password_field) = password.value().attr("name") else {
            continue;
        };
        let password_position = inputs
            .iter()
            .position(|input| std::ptr::eq(input.value(), password.value()))
            .unwrap_or(inputs.len());
        let username = inputs.iter().take(password_position).find(|input| {
            let kind = input
                .value()
                .attr("type")
                .unwrap_or("text")
                .to_ascii_lowercase();
            matches!(kind.as_str(), "text" | "email") && input.value().attr("name").is_some()
        });
        let Some(username_field) = username.and_then(|input| input.value().attr("name")) else {
            continue;
        };
        let mut extra_fields = Vec::new();
        for input in &inputs {
            let Some(name) = input.value().attr("name") else {
                continue;
            };
            if name == username_field || name == password_field {
                continue;
            }
            let kind = input
                .value()
                .attr("type")
                .unwrap_or("text")
                .to_ascii_lowercase();
            // Hidden values and the submit button travel with the form; unrelated text boxes,
            // checkboxes and the like are left out.
            if matches!(kind.as_str(), "hidden" | "submit") {
                extra_fields.push((
                    name.to_string(),
                    input.value().attr("value").unwrap_or_default().to_string(),
                ));
            }
        }
        let action =
            resolve_action(page_url, form.value().attr("action")).ok_or(LoginProblem::NoForm)?;
        return Ok(LoginForm {
            action,
            username_field: username_field.to_string(),
            password_field: password_field.to_string(),
            extra_fields,
        });
    }
    Err(LoginProblem::NoForm)
}

/// The session a successful sign-in gives.
#[derive(Clone, PartialEq, Eq)]
pub struct Session {
    /// A `Cookie` header value: every cookie the site set, `uid` and `pass` among them.
    pub cookie_header: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("cookie_header", &"[REDACTED]")
            .finish()
    }
}

/// Cookies from `Set-Cookie` header values. A cookie being deleted (empty, `deleted`, or expired
/// by `Max-Age=0`) is dropped. Later values replace earlier ones of the same name.
pub fn cookies_from_set_cookie<'a>(
    headers: impl IntoIterator<Item = &'a str>,
) -> Vec<(String, String)> {
    let mut cookies: Vec<(String, String)> = Vec::new();
    for header in headers {
        let mut parts = header.split(';');
        let Some((name, value)) = parts.next().and_then(|pair| pair.split_once('=')) else {
            continue;
        };
        let (name, value) = (name.trim().to_string(), value.trim().to_string());
        if name.is_empty() {
            continue;
        }
        let removed = value.is_empty()
            || value.eq_ignore_ascii_case("deleted")
            || parts.any(|attribute| {
                let attribute = attribute.trim().to_ascii_lowercase();
                attribute == "max-age=0" || attribute.starts_with("max-age=-")
            });
        cookies.retain(|(existing, _)| existing != &name);
        if !removed {
            cookies.push((name, value));
        }
    }
    cookies
}

/// A session from the cookies, if they are a signed-in browser's: `uid` and `pass` both present.
pub fn session_from_cookies(cookies: &[(String, String)]) -> Option<Session> {
    let has = |name: &str| {
        cookies
            .iter()
            .any(|(cookie, value)| cookie == name && !value.is_empty())
    };
    if !(has("uid") && has("pass")) {
        return None;
    }
    Some(Session {
        cookie_header: cookies
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; "),
    })
}

/// The error text on a page that showed the login form again, if it says something.
pub fn login_error_text(html: &str) -> String {
    let document = Html::parse_document(html);
    for selector in [
        ".error",
        ".alert",
        ".alert-danger",
        ".err",
        "td.text",
        "p.error",
    ] {
        let Ok(selector) = Selector::parse(selector) else {
            continue;
        };
        for element in document.select(&selector) {
            let text = element.text().collect::<Vec<_>>().join(" ");
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let lower = text.to_ascii_lowercase();
            if !text.is_empty()
                && text.len() < 160
                && [
                    "invalid",
                    "incorrect",
                    "failed",
                    "wrong",
                    "disabled",
                    "banned",
                    "not found",
                ]
                .iter()
                .any(|word| lower.contains(word))
            {
                return text;
            }
        }
    }
    String::new()
}

/// What the answer to the posted form means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    Signed(Session),
    Failed(LoginProblem),
}

/// Judge the answer to the posted login form from its status, redirect target, cookies and body.
pub fn judge_login_response(
    status: u16,
    location: Option<&str>,
    cookies: &[(String, String)],
    body: &str,
) -> LoginOutcome {
    if status == 429 {
        return LoginOutcome::Failed(LoginProblem::RateLimited);
    }
    if let Some(block) = detect_block(body) {
        return LoginOutcome::Failed(block);
    }
    if matches!(status, 403 | 503) {
        return LoginOutcome::Failed(LoginProblem::Unrecognised(format!("HTTP {status}")));
    }
    if let Some(session) = session_from_cookies(cookies) {
        return LoginOutcome::Signed(session);
    }
    if let Some(target) = location {
        let lower = target.to_ascii_lowercase();
        if lower.contains("2fa") || lower.contains("two") || lower.contains("verify") {
            return LoginOutcome::Failed(LoginProblem::TwoFactor);
        }
        if lower.contains("login") {
            return LoginOutcome::Failed(LoginProblem::BadCredentials(String::new()));
        }
    }
    if body.to_ascii_lowercase().contains("type=\"password\"") {
        return LoginOutcome::Failed(LoginProblem::BadCredentials(login_error_text(body)));
    }
    LoginOutcome::Failed(LoginProblem::Unrecognised(format!(
        "HTTP {status}, no session cookies"
    )))
}

// ---------------------------------------------------------------------------
// The account's RSS link
// ---------------------------------------------------------------------------

/// The personal RSS link on the site's RSS page (`getrss.php`): the first `t.rss?...` address in
/// the page, which carries the account's `u=` and `tp=` parameters. If the page only shows the
/// two values, the link is built from them. `None` when the page has neither.
///
/// Inferred, not verified: that the page prints the link. Returns the link in full, passkey and
/// all: it is a secret and goes straight into an encrypted feed, never to a client or a log.
pub fn parse_rss_link(html: &str, site_link: &str) -> Option<String> {
    static LINK: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r#"https?://[^\s"'<>]*t\.rss\?[^\s"'<>]*"#).expect("rss link pattern")
    });
    static UID: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"\bu=(\d+)").expect("uid pattern"));
    static PASS: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"\btp=([0-9a-fA-F]{16,})").expect("passkey pattern")
    });
    let text = html.replace("&amp;", "&");
    if let Some(found) = LINK.find(&text)
        && found.as_str().contains("tp=")
    {
        return Some(found.as_str().to_string());
    }
    let uid = UID.captures(&text)?.get(1)?.as_str().to_string();
    let pass = PASS.captures(&text)?.get(1)?.as_str().to_string();
    let base = if site_link.ends_with('/') {
        site_link.to_string()
    } else {
        format!("{site_link}/")
    };
    Some(format!("{base}t.rss?u={uid};tp={pass};download"))
}

/// Read the account's RSS link using a signed-in session.
pub async fn fetch_rss_link(
    site_link: &str,
    cookie_header: &str,
    user_agent: &str,
) -> anyhow::Result<Option<String>> {
    use crate::services::http_client::{
        INDEXER_RESPONSE_LIMIT, OutboundHttpProfile, outbound_client_builder, response_text_limited,
    };
    let client = outbound_client_builder(OutboundHttpProfile::Indexer)
        .user_agent(if user_agent.trim().is_empty() {
            DEFAULT_USER_AGENT
        } else {
            user_agent
        })
        .build()?;
    let base = if site_link.ends_with('/') {
        site_link.to_string()
    } else {
        format!("{site_link}/")
    };
    let response = client
        .get(format!("{base}getrss.php"))
        .header(reqwest::header::COOKIE, cookie_header)
        .send()
        .await
        .map_err(|error| {
            anyhow::anyhow!("the RSS page could not be reached: {}", error.without_url())
        })?;
    let html = response_text_limited(response, INDEXER_RESPONSE_LIMIT).await?;
    if !html.contains("/lout.php") {
        anyhow::bail!("the site showed the RSS page signed out");
    }
    Ok(parse_rss_link(&html, &base))
}

/// A browser-like user agent for sign-ins when none is configured. Cloudflare ties its clearance
/// cookie to the agent, so the same one has to be used for the session afterwards.
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

/// Sign in. `Ok(Err(problem))` is the site's answer (refused, blocked, challenged); `Err` is a
/// failure to reach it at all, which says nothing about the credentials and costs no attempt.
///
/// One request for the login page and one post: never a retry.
pub async fn sign_in(
    site_link: &str,
    username: &str,
    password: &str,
    user_agent: &str,
) -> anyhow::Result<Result<Session, LoginProblem>> {
    use crate::services::http_client::{
        INDEXER_RESPONSE_LIMIT, OutboundHttpProfile, outbound_client_builder, response_text_limited,
    };
    use reqwest::header;

    let client = outbound_client_builder(OutboundHttpProfile::Indexer)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(if user_agent.trim().is_empty() {
            DEFAULT_USER_AGENT
        } else {
            user_agent
        })
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;
    let base = if site_link.ends_with('/') {
        site_link.to_string()
    } else {
        format!("{site_link}/")
    };
    let cookies_of = |response: &reqwest::Response| {
        cookies_from_set_cookie(
            response
                .headers()
                .get_all(header::SET_COOKIE)
                .iter()
                .filter_map(|value| value.to_str().ok()),
        )
    };

    // The login page: `login.php`, or the front page if that has no form.
    let login_url = format!("{base}login.php");
    let page = client.get(&login_url).send().await.map_err(|error| {
        anyhow::anyhow!(
            "the login page could not be reached: {}",
            error.without_url()
        )
    })?;
    let mut cookies = cookies_of(&page);
    let page_status = page.status().as_u16();
    let page_html = response_text_limited(page, INDEXER_RESPONSE_LIMIT).await?;
    if let Some(block) = detect_block(&page_html) {
        return Ok(Err(block));
    }
    if page_status == 429 {
        return Ok(Err(LoginProblem::RateLimited));
    }
    let (form, form_page) = match parse_login_form(&page_html, &login_url) {
        Ok(form) => (form, login_url.clone()),
        Err(_) => {
            let front = client.get(&base).send().await.map_err(|error| {
                anyhow::anyhow!("the site could not be reached: {}", error.without_url())
            })?;
            cookies.extend(cookies_of(&front));
            let html = response_text_limited(front, INDEXER_RESPONSE_LIMIT).await?;
            if let Some(block) = detect_block(&html) {
                return Ok(Err(block));
            }
            match parse_login_form(&html, &base) {
                Ok(form) => (form, base.clone()),
                Err(problem) => return Ok(Err(problem)),
            }
        }
    };

    let encoded = form
        .filled(username, password)
        .iter()
        .map(|(name, value)| {
            format!(
                "{}={}",
                urlencoding::encode(name),
                urlencoding::encode(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let mut request = client
        .post(&form.action)
        .header(header::REFERER, &form_page)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(encoded);
    if !cookies.is_empty() {
        let cookie_header = cookies
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        request = request.header(header::COOKIE, cookie_header);
    }
    let answer = request.send().await.map_err(|error| {
        anyhow::anyhow!("the sign-in could not be sent: {}", error.without_url())
    })?;
    let status = answer.status().as_u16();
    let location = answer
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    cookies.extend(cookies_of(&answer));
    // Later cookies replace earlier ones of the same name; deletions drop them.
    let merged = cookies_from_set_cookie(
        cookies
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str),
    );
    let body = response_text_limited(answer, INDEXER_RESPONSE_LIMIT)
        .await
        .unwrap_or_default();
    Ok(
        match judge_login_response(status, location.as_deref(), &merged, &body) {
            LoginOutcome::Signed(session) => Ok(session),
            LoginOutcome::Failed(problem) => Err(problem),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORM: &str = include_str!("../../../../tests/fixtures/iptorrents/login_form.html");
    const CAPTCHA: &str = include_str!("../../../../tests/fixtures/iptorrents/login_captcha.html");
    const CLOUDFLARE: &str =
        include_str!("../../../../tests/fixtures/iptorrents/login_cloudflare.html");
    const TWO_FACTOR: &str = include_str!("../../../../tests/fixtures/iptorrents/login_2fa.html");
    const FAILED: &str = include_str!("../../../../tests/fixtures/iptorrents/login_failed.html");

    const RSS_PAGE: &str = include_str!("../../../../tests/fixtures/iptorrents/getrss.html");

    #[test]
    fn the_personal_rss_link_is_read_from_the_rss_page() {
        let link = parse_rss_link(RSS_PAGE, "https://iptorrents.com/").unwrap();
        assert_eq!(
            link,
            "https://iptorrents.com/t.rss?u=123456;tp=0123456789abcdef0123456789abcdef;48;5;download"
        );
        // When the page shows only the two values, the link is built from them.
        let bare = "<p>uid: u=777 and tp=abcdef0123456789abcdef</p>";
        assert_eq!(
            parse_rss_link(bare, "https://iptorrents.com").unwrap(),
            "https://iptorrents.com/t.rss?u=777;tp=abcdef0123456789abcdef;download"
        );
        assert!(parse_rss_link("<p>nothing here</p>", "https://iptorrents.com/").is_none());
        // And it never survives into a display form.
        let shown = crate::services::redact::feed_url_display(&link);
        assert!(!shown.contains("0123456789abcdef"), "{shown}");
    }

    #[test]
    fn the_login_form_is_read_from_the_page() {
        let form = parse_login_form(FORM, "https://iptorrents.com/login.php").unwrap();
        assert_eq!(form.action, "https://iptorrents.com/take_login.php");
        assert_eq!(form.username_field, "username");
        assert_eq!(form.password_field, "password");
        assert!(
            form.extra_fields
                .contains(&("returnto".into(), "/t".into()))
        );
        assert!(
            form.extra_fields
                .contains(&("csrf_token".into(), "sanitized-token-123".into()))
        );
        assert!(
            form.extra_fields
                .contains(&("submit".into(), "Log in".into()))
        );
    }

    #[test]
    fn filling_the_form_keeps_hidden_fields_and_adds_the_credentials() {
        let form = parse_login_form(FORM, "https://iptorrents.com/login.php").unwrap();
        let filled = form.filled("someone", "p@ss word");
        assert!(filled.contains(&("username".into(), "someone".into())));
        assert!(filled.contains(&("password".into(), "p@ss word".into())));
        assert!(filled.contains(&("csrf_token".into(), "sanitized-token-123".into())));
    }

    #[test]
    fn a_relative_or_missing_action_is_resolved_against_the_page() {
        let html = r#"<form><input name="u" type="text"><input name="p" type="password"></form>"#;
        let form = parse_login_form(html, "https://iptorrents.com/login.php").unwrap();
        assert_eq!(form.action, "https://iptorrents.com/login.php");
        assert_eq!(
            (form.username_field.as_str(), form.password_field.as_str()),
            ("u", "p")
        );
    }

    #[test]
    fn a_page_without_a_password_form_has_no_form() {
        assert_eq!(
            parse_login_form(
                "<html><form><input name='q'></form></html>",
                "https://x.example/"
            )
            .unwrap_err(),
            LoginProblem::NoForm
        );
    }

    #[test]
    fn captcha_cloudflare_and_two_factor_pages_are_each_recognised() {
        assert_eq!(detect_block(CAPTCHA), Some(LoginProblem::Captcha));
        assert_eq!(detect_block(CLOUDFLARE), Some(LoginProblem::Cloudflare));
        assert_eq!(detect_block(TWO_FACTOR), Some(LoginProblem::TwoFactor));
        assert_eq!(detect_block(FORM), None);
        assert_eq!(detect_block(FAILED), None);
        for problem in [
            LoginProblem::Captcha,
            LoginProblem::Cloudflare,
            LoginProblem::TwoFactor,
        ] {
            assert!(problem.message().contains("cookie"), "{problem:?}");
            assert!(problem.is_blocked());
        }
    }

    #[test]
    fn cookies_are_captured_and_deleted_ones_dropped() {
        let cookies = cookies_from_set_cookie([
            "uid=12345; expires=Wed, 01 Jan 2031 00:00:00 GMT; Max-Age=31536000; path=/",
            "pass=abcdef0123; path=/; secure; HttpOnly",
            "PHPSESSID=old; path=/",
            "PHPSESSID=deleted; path=/; Max-Age=0",
            "cf_clearance=zzz; path=/",
        ]);
        assert_eq!(
            cookies,
            vec![
                ("uid".to_string(), "12345".to_string()),
                ("pass".to_string(), "abcdef0123".to_string()),
                ("cf_clearance".to_string(), "zzz".to_string()),
            ]
        );
        let session = session_from_cookies(&cookies).unwrap();
        assert_eq!(
            session.cookie_header,
            "uid=12345; pass=abcdef0123; cf_clearance=zzz"
        );
        // The debug output never shows the cookie values.
        assert!(!format!("{session:?}").contains("abcdef0123"));
    }

    #[test]
    fn a_session_needs_both_uid_and_pass() {
        assert!(session_from_cookies(&[("uid".into(), "1".into())]).is_none());
        assert!(
            session_from_cookies(&[("uid".into(), "1".into()), ("pass".into(), String::new())])
                .is_none()
        );
    }

    #[test]
    fn a_redirect_with_session_cookies_is_a_sign_in() {
        let cookies = cookies_from_set_cookie(["uid=1; path=/", "pass=2; path=/"]);
        let outcome = judge_login_response(302, Some("/t"), &cookies, "");
        assert!(matches!(outcome, LoginOutcome::Signed(_)));
    }

    #[test]
    fn a_page_that_shows_the_form_again_is_a_refusal_with_the_sites_words() {
        match judge_login_response(200, None, &[], FAILED) {
            LoginOutcome::Failed(LoginProblem::BadCredentials(detail)) => {
                assert!(detail.contains("Invalid username or password"), "{detail}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn blocks_win_over_a_missing_cookie_and_limits_are_reported() {
        assert_eq!(
            judge_login_response(200, None, &[], CAPTCHA),
            LoginOutcome::Failed(LoginProblem::Captcha)
        );
        assert_eq!(
            judge_login_response(503, None, &[], CLOUDFLARE),
            LoginOutcome::Failed(LoginProblem::Cloudflare)
        );
        assert_eq!(
            judge_login_response(302, Some("/2fa.php"), &[], ""),
            LoginOutcome::Failed(LoginProblem::TwoFactor)
        );
        assert_eq!(
            judge_login_response(429, None, &[], ""),
            LoginOutcome::Failed(LoginProblem::RateLimited)
        );
        assert!(matches!(
            judge_login_response(200, None, &[], "<html>welcome</html>"),
            LoginOutcome::Failed(LoginProblem::Unrecognised(_))
        ));
    }
}
