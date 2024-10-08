use std::fmt::{self, Debug, Display, Formatter};

use regex::Regex;

#[derive(Debug)]
pub enum ParseRepoError {
    NotSSH(String),
    CantParseColon(String),
    CantFindProjectAndName(String),
    UnparseableHTTPURL(String),
    InvalidRegexp(regex::Error),
}

impl Display for ParseRepoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ParseRepoError::NotSSH(url) => {
                write!(f, "Invalid repository URL: not SSH: {}", url)
            }
            ParseRepoError::CantParseColon(url) => {
                write!(
                    f,
                    "Invalid repository URL: cannot parse colon separator: {}",
                    url
                )
            }
            ParseRepoError::CantFindProjectAndName(url) => {
                write!(
                    f,
                    "Invalid repository URL: cannot find project and name: {}",
                    url
                )
            }
            ParseRepoError::UnparseableHTTPURL(url) => {
                write!(f, "Invalid repository URL: unparseable HTTP URL: {}", url)
            }
            ParseRepoError::InvalidRegexp(e) => {
                write!(f, "Invalid repository URL: invalid regexp: {}", e)
            }
        }
    }
}

impl From<CantConvertError> for ParseRepoError {
    fn from(err: CantConvertError) -> Self {
        match err {
            CantConvertError::InvalidURL(url) => ParseRepoError::UnparseableHTTPURL(url),
            CantConvertError::MissingOrganization(url) => {
                ParseRepoError::CantFindProjectAndName(url)
            }
            CantConvertError::MissingProject(url) => ParseRepoError::CantFindProjectAndName(url),
            CantConvertError::InvalidRegexp(e) => ParseRepoError::InvalidRegexp(e),
        }
    }
}

impl From<CantConvertSSHError> for ParseRepoError {
    fn from(err: CantConvertSSHError) -> Self {
        match err {
            CantConvertSSHError::NotSSH(url) => ParseRepoError::NotSSH(url),
            CantConvertSSHError::CantParseColon(url) => ParseRepoError::CantParseColon(url),
            CantConvertSSHError::CantFindProjectAndName(url) => {
                ParseRepoError::CantFindProjectAndName(url)
            }
        }
    }
}

pub fn repository(repo_url: String) -> Result<(String, String, String), ParseRepoError> {
    if repo_url.contains('@') && repo_url.contains(':') {
        return parse_ssh_url(&repo_url).map_err(ParseRepoError::from);
    }

    parse_http_url(&repo_url).map_err(ParseRepoError::from)
}

#[derive(Debug)]
enum CantConvertError {
    InvalidURL(String),
    MissingOrganization(String),
    MissingProject(String),
    InvalidRegexp(regex::Error),
}

#[derive(Debug)]
enum CantConvertSSHError {
    NotSSH(String),
    CantParseColon(String),
    CantFindProjectAndName(String),
}

fn parse_ssh_url(url: &str) -> Result<(String, String, String), CantConvertSSHError> {
    let parts: Vec<&str> = url.splitn(2, '@').collect();

    if parts.len() != 2 {
        return Err(CantConvertSSHError::NotSSH(url.to_string()));
    }

    let repo_path = parts[1];
    let parts: Vec<&str> = repo_path.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(CantConvertSSHError::CantParseColon(url.to_string()));
    }

    let host = parts[0];
    let path_parts: Vec<&str> = parts[1].splitn(2, '/').collect();
    if path_parts.len() != 2 {
        return Err(CantConvertSSHError::CantFindProjectAndName(url.to_string()));
    }

    let team = path_parts[0];
    let project = path_parts[1].strip_suffix(".git").unwrap_or(path_parts[1]);

    Ok((host.to_string(), team.to_string(), project.to_string()))
}

fn parse_http_url(url: &str) -> Result<(String, String, String), CantConvertError> {
    let re = Regex::new(r"^(https://)?(github\.com/)?(?<org>[a-zA-Z0-9-]+)/(?<repo>[\w\.-]+).*$")
        .map_err(CantConvertError::InvalidRegexp)?;

    let caps = re
        .captures(url)
        .ok_or(CantConvertError::InvalidURL(url.to_owned()))?;
    let team = caps
        .name("org")
        .ok_or(CantConvertError::MissingOrganization(url.to_owned()))?
        .as_str();
    let project = caps
        .name("repo")
        .ok_or(CantConvertError::MissingProject(url.to_owned()))?
        .as_str()
        .trim_end_matches(".git");

    Ok((
        "github.com".to_string(),
        team.to_string(),
        project.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        let cases = vec![
            (
                "git@github.com:example/application.git",
                ("github.com", "example", "application"),
            ),
            (
                "github.com/example/application",
                ("github.com", "example", "application"),
            ),
            (
                "example/application",
                ("github.com", "example", "application"),
            ),
            (
                "https://github.com/example/application",
                ("github.com", "example", "application"),
            ),
            (
                "https://github.com/example/application/issues",
                ("github.com", "example", "application"),
            ),
            (
                "https://github.com/example/application/security/dependabot",
                ("github.com", "example", "application"),
            ),
            (
                "https://github.com/example/application/this/is/a/made/up/path",
                ("github.com", "example", "application"),
            ),
        ];

        for (input, expected) in cases {
            let (host, team, project) = repository(input.to_string()).unwrap();
            let (expected_host, expected_team, expected_project) = expected;
            assert_eq!(host, expected_host.to_string());
            assert_eq!(team, expected_team.to_string());
            assert_eq!(project, expected_project.to_string());
        }
    }

    #[test]
    fn test_invalid_url() {
        let cases = vec![""];

        for input in cases {
            let result = repository(input.to_string());
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_valid_http_conversor() {
        let cases = vec![
            (
                "https://github.com/patrickdappollonio/gc-rust",
                false,
                ("github.com", "patrickdappollonio", "gc-rust"),
            ),
            (
                "https://github.com/patrickdappollonio/gc-rust.git",
                false,
                ("github.com", "patrickdappollonio", "gc-rust"),
            ),
            ("http://patrickdap.com", true, ("", "", "")),
            (
                "https://github.com/patrickdappollonio/gc-rust/foo/bar",
                false,
                ("github.com", "patrickdappollonio", "gc-rust"),
            ),
            (
                "patrickdappollonio/gc-rust",
                false,
                ("github.com", "patrickdappollonio", "gc-rust"),
            ),
        ];

        for (input, should_fail, expected) in cases {
            let result = parse_http_url(input);

            if should_fail {
                assert!(result.is_err());
                continue;
            }

            let (expected_host, expected_team, expected_project) = expected;
            let (host, team, project) = result.unwrap();
            assert_eq!(host, expected_host.to_string());
            assert_eq!(team, expected_team.to_string());
            assert_eq!(project, expected_project.to_string());
        }
    }

    #[test]
    fn test_valid_ssh_conversor() {
        let cases = vec![(
            "git@github.com:team/project.git",
            false,
            ("github.com", "team", "project"),
        )];

        for (input, should_fail, expected) in cases {
            let result = parse_ssh_url(input);

            if should_fail {
                assert!(result.is_err());
                continue;
            }

            let (expected_host, expected_team, expected_project) = expected;
            let (host, team, project) = result.unwrap();
            assert_eq!(host, expected_host.to_string());
            assert_eq!(team, expected_team.to_string());
            assert_eq!(project, expected_project.to_string());
        }
    }
}
