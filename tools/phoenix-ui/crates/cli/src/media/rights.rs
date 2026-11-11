use regex::Regex;
use std::sync::LazyLock;

static REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"non-?commercial|by-nc|\bnc\b|no ?deriv|by-nd|\bnd\b|requires licensing")
        .expect("valid license pattern")
});
static CHECK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"third-party rights|not verified|not specified|in copyright")
        .expect("valid license pattern")
});
static PAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"cc0|publicdomain|public domain|\bpdm?\b|no known restrictions|not subject to us copyright|creativecommons\.org/licenses/by(-sa)?/|\bcc[ -]by(-sa)?\b|attribution( |-sharealike )\d").expect("valid license pattern")
});

pub(super) fn use_grade(license: &str, license_url: &str) -> &'static str {
    let text = format!("{license} {license_url}").to_lowercase();
    if REFERENCE.is_match(&text) {
        "reference"
    } else if CHECK.is_match(&text) {
        "check"
    } else if PAGE.is_match(&text) {
        "page"
    } else {
        "check"
    }
}

pub(super) fn use_label(grade: &str) -> &'static str {
    match grade {
        "page" => "可上线",
        "reference" => "仅参考",
        _ => "需核对",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restrictions_win_over_open_license_words() {
        for (license, expected) in [
            ("CC0", "page"), ("PDM", "page"), ("CC BY 4.0", "page"),
            ("CC BY-SA 4.0", "page"), ("CC BY-NC 4.0", "reference"),
            ("CC BY-ND", "reference"), ("Public domain; non-commercial", "reference"),
            ("V&A terms: permitted non-commercial uses only; commercial/home-page/high-resolution publication requires licensing", "reference"),
            ("NASA media usage guidelines; generally not subject to US copyright; verify third-party credits", "page"),
            ("Third-party rights mentioned; check item credits", "check"),
            ("Rights not specified; check item", "check"), ("", "check"),
        ] {
            assert_eq!(use_grade(license, ""), expected, "{license}");
        }
        assert_eq!(
            use_grade("", "https://creativecommons.org/licenses/by-sa/4.0/"),
            "page"
        );
        assert_eq!(use_grade("CC0 not verified", ""), "check");
        assert_eq!(use_grade("Attribution-ShareAlike 4.0", ""), "page");
    }
}
