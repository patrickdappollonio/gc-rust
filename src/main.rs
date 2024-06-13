use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::{env, fmt};

enum ApplicationError {
    BaseDirNotFound,
    BaseDirCannotBeOpened(std::io::Error),
    CantCreateTargetDir(std::io::Error),
    FailedCloneCommand(std::io::Error),
    FailedGitOperation(String),
}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ApplicationError::BaseDirNotFound => write!(f, "Base directory not found"),
            ApplicationError::BaseDirCannotBeOpened(err) => {
                write!(f, "Base directory cannot be opened: {}", err)
            }
            ApplicationError::CantCreateTargetDir(err) => {
                write!(f, "Cannot create target directory: {}", err)
            }
            ApplicationError::FailedCloneCommand(err) => {
                write!(f, "Failed to run the git clone command: {}", err)
            }
            ApplicationError::FailedGitOperation(err) => {
                write!(f, "Failed to run the git operation: {}", err)
            }
        }
    }
}

impl From<ParseRepoError> for ApplicationError {
    fn from(err: ParseRepoError) -> Self {
        ApplicationError::FailedGitOperation(err.to_string())
    }
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), ApplicationError> {
    // Get the base directory
    let base_dir = env::var("GOPATH").map_err(|_| ApplicationError::BaseDirNotFound)?;

    // Try opening the base directory
    fs::read_dir(&base_dir).map_err(ApplicationError::BaseDirCannotBeOpened)?;

    // Get the repository URL from the command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: gc <repository-url>");
        return Ok(());
    }
    let repo_url = &args[1];

    // Parse the repository URL
    let (host, team, project) = parse_repo_url(repo_url.to_string())?;
    let project_path = format!("{}/{}/{}/{}", base_dir, host, team, project);

    // Create the directory if it does not exist
    if !Path::new(&project_path).exists() {
        fs::create_dir_all(&project_path).map_err(ApplicationError::CantCreateTargetDir)?;
    }

    // Run the git clone command
    let exec = Command::new("git")
        .arg("clone")
        .arg(repo_url)
        .arg(&project_path)
        .output()
        .map_err(ApplicationError::FailedCloneCommand)?;

    if !exec.status.success() {
        let stderr = String::from_utf8_lossy(&exec.stderr);
        return Err(ApplicationError::FailedGitOperation(stderr.into_owned()));
    }

    eprintln!("Successfully cloned {} into {}", repo_url, project_path);
    println!("{}", project_path);
    Ok(())
}

enum ParseRepoError {
    NotSSH(String),
    CantParseColon(String),
    CantFindProjectAndName(String),
}

impl Display for ParseRepoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ParseRepoError::NotSSH(url) => {
                write!(f, "Invalid repository URL: {}", url)
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
        }
    }
}

fn parse_repo_url(repo_url: String) -> Result<(String, String, String), ParseRepoError> {
    let parts: Vec<&str> = repo_url.split('@').collect();
    if parts.len() != 2 {
        return Err(ParseRepoError::NotSSH(repo_url));
    }

    let repo_path = parts[1];
    let parts: Vec<&str> = repo_path.split(':').collect();
    if parts.len() != 2 {
        return Err(ParseRepoError::CantParseColon(repo_url));
    }

    let host = parts[0];
    let path_parts: Vec<&str> = parts[1].split('/').collect();
    if path_parts.len() != 2 {
        return Err(ParseRepoError::CantFindProjectAndName(repo_url));
    }

    let team = path_parts[0];
    let project = path_parts[1].replace(".git", "");

    Ok((host.to_string(), team.to_string(), project))
}
