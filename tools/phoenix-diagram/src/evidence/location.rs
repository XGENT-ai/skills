// Repository identity and forge links, ported from the frozen Archify 3.1.0
// renderers/shared/repository-location.mjs. Nothing here contacts a remote.
use url::Url;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Location {
    /// Hostname, endpoint, path kind and path: equal identities name one repository.
    pub identity: (String, String, String, String),
    pub url: String,
    pub provider: Option<&'static str>,
    pub protocol: String,
    pub path: String,
    pub endpoint: String,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Rejection {
    Invalid,
    /// HTTP(S) userinfo in an authored URL.
    Credentials,
}

const PROVIDER_HOSTS: [(&str, &str); 3] = [
    ("github.com", "github"),
    ("gitee.com", "gitee"),
    ("gitlab.com", "gitlab"),
];

// ECMAScript `\s`, which differs from Unicode White_Space at U+0085 and U+FEFF.
fn js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn forbidden(c: char) -> bool {
    js_space(c) || c == '\\' || c.is_ascii_control() || c == '?' || c == '#'
}

fn decode_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// ECMAScript `encodeURIComponent`.
pub(crate) fn encode_component(value: &str) -> String {
    let mut result = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

/// `parseRepositoryRemote`. Authored values are taken literally and may not
/// carry HTTP(S) credentials; origin values are trimmed first.
pub(crate) fn parse_remote(
    value: &str,
    authored: bool,
    provider_hint: Option<&str>,
) -> Result<Location, Rejection> {
    let raw = if authored { value } else { value.trim() };
    if raw.is_empty() || raw.chars().any(forbidden) {
        return Err(Rejection::Invalid);
    }
    let scp = raw
        .strip_prefix("git@")
        .and_then(|rest| rest.split_once(':'))
        .filter(|(host, path)| !host.is_empty() && !host.contains('/') && !path.is_empty());
    let scp_absolute = scp.is_some_and(|(_, path)| path.starts_with('/'));
    let expanded = match scp {
        Some((host, path)) => format!("ssh://git@{host}/{}", path.trim_start_matches('/')),
        None => raw.to_owned(),
    };
    let (scheme, rest) = expanded.split_once("://").ok_or(Rejection::Invalid)?;
    if !["http", "https", "ssh"].contains(&scheme.to_ascii_lowercase().as_str()) {
        return Err(Rejection::Invalid);
    }
    let (_, remote_path) = rest.split_once('/').ok_or(Rejection::Invalid)?;
    if rest.starts_with('/') || remote_path.is_empty() {
        return Err(Rejection::Invalid);
    }
    // Validate the original path before URL parsing can collapse dot segments.
    let trimmed = remote_path.strip_suffix('/').unwrap_or(remote_path);
    let mut segments = trimmed
        .split('/')
        .map(|part| {
            if scp.is_some() {
                Some(part.to_owned())
            } else {
                decode_component(part)
            }
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(Rejection::Invalid)?;
    if segments.iter().any(|part| {
        part.is_empty()
            || part == "."
            || part == ".."
            || part.chars().any(|c| c == '/' || forbidden(c))
    }) {
        return Err(Rejection::Invalid);
    }
    let url = Url::parse(&expanded).map_err(|_| Rejection::Invalid)?;
    let protocol = format!("{}:", url.scheme());
    if protocol == "ssh:" && (url.username() != "git" || url.password().is_some()) {
        return Err(Rejection::Invalid);
    }
    if authored && protocol != "ssh:" && (!url.username().is_empty() || url.password().is_some()) {
        return Err(Rejection::Credentials);
    }
    let hostname = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if hostname.is_empty() {
        return Err(Rejection::Invalid);
    }
    let provider = PROVIDER_HOSTS
        .iter()
        .find(|(host, _)| *host == hostname)
        .map(|(_, provider)| *provider)
        .or((provider_hint == Some("gitlab")).then_some("gitlab"));
    let last = segments.len() - 1;
    if let Some(provider) = provider {
        let name = &segments[last];
        let suffix = name.len().checked_sub(4).filter(|at| {
            name.is_char_boundary(*at)
                && if provider == "github" {
                    name[*at..].eq_ignore_ascii_case(".git")
                } else {
                    &name[*at..] == ".git"
                }
        });
        if let Some(at) = suffix {
            segments[last] = name[..at].to_owned();
        }
    }
    if segments[last].is_empty() || segments[last] == "." || segments[last] == ".." {
        return Err(Rejection::Invalid);
    }
    let path = segments.join("/");
    let port = url.port().map(|port| port.to_string()).unwrap_or_else(|| {
        match protocol.as_str() {
            "ssh:" => "22",
            "https:" => "443",
            _ => "80",
        }
        .into()
    });
    let standard = provider.is_some()
        && ((protocol == "https:" && port == "443") || (protocol == "ssh:" && port == "22"));
    let endpoint = if standard {
        "standard".to_owned()
    } else {
        format!("{protocol}{port}")
    };
    let path_kind = if provider.is_some() {
        "repository"
    } else if scp.is_some() && !scp_absolute {
        "relative"
    } else {
        "absolute"
    };
    let identity_path = if matches!(provider, Some("github" | "gitlab")) {
        path.to_lowercase()
    } else {
        path.clone()
    };
    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
        None => url.host_str().unwrap_or_default().to_owned(),
    };
    let encoded: Vec<_> = segments.iter().map(|part| encode_component(part)).collect();
    let canonical = if scp.is_some() {
        format!(
            "git@{hostname}:{}{path}",
            if scp_absolute { "/" } else { "" }
        )
    } else {
        format!(
            "{protocol}//{}{host}/{}",
            if protocol == "ssh:" { "git@" } else { "" },
            encoded.join("/")
        )
    };
    Ok(Location {
        identity: (hostname, endpoint.clone(), path_kind.into(), identity_path),
        url: canonical,
        provider,
        protocol,
        path,
        endpoint,
    })
}

/// `redactRepositoryRemote`: removes userinfo, query and fragment for diagnostics.
pub(crate) fn redact(value: &str) -> String {
    let mut result = value.to_owned();
    if let Some(cut) = result.find(['?', '#']) {
        result.truncate(cut);
        result.push_str("?REDACTED");
    }
    let lower = result.to_ascii_lowercase();
    if let Some(scheme) = ["https://", "http://", "ssh://"]
        .into_iter()
        .find(|scheme| lower.starts_with(scheme))
    {
        let authority_end = result[scheme.len()..]
            .find('/')
            .map_or(result.len(), |at| at + scheme.len());
        if let Some(at) = result[scheme.len()..authority_end].rfind('@') {
            result.replace_range(scheme.len()..scheme.len() + at, "REDACTED");
        }
    }
    result
}

pub(crate) fn source_href(
    provider: &str,
    url: &str,
    revision: &str,
    path: &str,
    line: Option<u64>,
    end_line: Option<u64>,
) -> String {
    let encoded: Vec<_> = path.split('/').map(encode_component).collect();
    let encoded = encoded.join("/");
    let end = match end_line.filter(|end| Some(*end) != line) {
        Some(end) => format!("-{}{end}", if provider == "github" { "L" } else { "" }),
        None => String::new(),
    };
    let fragment = line.map_or(String::new(), |line| format!("#L{line}{end}"));
    if provider != "gitlab" {
        return format!("{url}/blob/{revision}/{encoded}{fragment}");
    }
    // GitLab renders Markdown blobs, where line anchors select nothing; the
    // plain view keeps the cited lines highlighted.
    let lower = path.to_ascii_lowercase();
    let plain = if line.is_some() && (lower.ends_with(".md") || lower.ends_with(".markdown")) {
        "?plain=1"
    } else {
        ""
    };
    format!("{url}/-/blob/{revision}/{encoded}{plain}{fragment}")
}

pub(crate) fn tree_href(provider: &str, url: &str, revision: &str) -> String {
    format!(
        "{url}/{}tree/{revision}",
        if provider == "gitlab" { "-/" } else { "" }
    )
}

/// GitLab projects may sit in nested groups; GitHub and Gitee are owner/repository.
pub(crate) fn web_path(provider: &str, path: &str) -> bool {
    let segments: Vec<_> = path.split('/').collect();
    let valid = |part: &&str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    };
    segments.iter().all(valid)
        && if provider == "gitlab" {
            segments.len() >= 2
        } else {
            segments.len() == 2
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_ssh_and_scp_forms_of_one_forge_repository_share_an_identity() {
        let https = parse_remote("https://github.com/XGENT-ai/skills.git", true, None).unwrap();
        let scp = parse_remote("git@github.com:xgent-ai/skills", true, None).unwrap();
        let ssh = parse_remote("ssh://git@github.com/XGENT-ai/Skills.GIT/", true, None).unwrap();
        assert_eq!(https.identity, scp.identity);
        assert_eq!(https.identity, ssh.identity);
        assert_eq!(https.url, "https://github.com/XGENT-ai/skills");
        assert_eq!(scp.url, "git@github.com:xgent-ai/skills");
        assert_eq!(https.endpoint, "standard");
        let gitee = parse_remote("https://gitee.com/Owner/Repo.GIT", true, None).unwrap();
        assert_eq!(gitee.path, "Owner/Repo.GIT");
    }

    #[test]
    fn credentials_dot_segments_queries_and_foreign_ssh_users_are_rejected() {
        assert_eq!(
            parse_remote("https://user:token@github.com/a/b", true, None),
            Err(Rejection::Credentials)
        );
        assert!(parse_remote("https://user:token@github.com/a/b", false, None).is_ok());
        for value in [
            "https://github.com/a/../b",
            "https://github.com/a//b",
            "https://github.com/a/b?x=1",
            "https://github.com/a/b#x",
            "https://github.com/a/%2e%2e",
            "https://github.com/a/b c",
            "ssh://root@github.com/a/b",
            "ssh://git:pw@github.com/a/b",
            "ftp://github.com/a/b",
            "/home/me/repo",
            "github.com/a/b",
        ] {
            assert_eq!(
                parse_remote(value, true, None),
                Err(Rejection::Invalid),
                "{value}"
            );
        }
        assert_eq!(
            redact("https://user:token@github.com/a/b?token=1"),
            "https://REDACTED@github.com/a/b?REDACTED"
        );
    }

    #[test]
    fn self_managed_gitlab_needs_the_hint_and_keeps_its_port_endpoint() {
        let plain = parse_remote("https://git.example.com/group/sub/repo.git", true, None).unwrap();
        assert_eq!(plain.provider, None);
        assert_eq!(plain.path, "group/sub/repo.git");
        let hinted = parse_remote(
            "https://git.example.com/group/sub/repo.git",
            true,
            Some("gitlab"),
        )
        .unwrap();
        assert_eq!(
            (hinted.provider, hinted.path.as_str()),
            (Some("gitlab"), "group/sub/repo")
        );
        let port = parse_remote("https://gitlab.com:8443/a/b", true, None).unwrap();
        assert_eq!(port.endpoint, "https:8443");
        assert_eq!(port.url, "https://gitlab.com:8443/a/b");
    }

    #[test]
    fn source_links_follow_each_forge_line_anchor_syntax() {
        let sha = "a".repeat(40);
        assert_eq!(
            source_href(
                "github",
                "https://github.com/o/r",
                &sha,
                "src/a b.rs",
                Some(3),
                Some(5)
            ),
            format!("https://github.com/o/r/blob/{sha}/src/a%20b.rs#L3-L5")
        );
        assert_eq!(
            source_href(
                "gitee",
                "https://gitee.com/o/r",
                &sha,
                "a.rs",
                Some(3),
                Some(5)
            ),
            format!("https://gitee.com/o/r/blob/{sha}/a.rs#L3-5")
        );
        assert_eq!(
            source_href(
                "gitlab",
                "https://gitlab.com/g/r",
                &sha,
                "README.md",
                Some(2),
                Some(2)
            ),
            format!("https://gitlab.com/g/r/-/blob/{sha}/README.md?plain=1#L2")
        );
        assert_eq!(
            source_href("github", "https://github.com/o/r", &sha, "a.rs", None, None),
            format!("https://github.com/o/r/blob/{sha}/a.rs")
        );
        assert_eq!(
            tree_href("gitlab", "https://gitlab.com/g/r", &sha),
            format!("https://gitlab.com/g/r/-/tree/{sha}")
        );
        assert!(web_path("github", "o/r") && !web_path("github", "o/r/x"));
        assert!(web_path("gitlab", "g/s/r") && !web_path("gitlab", "r"));
    }
}
