use emoji::Emoji;
use getopts::Options;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::{env, fmt};
use std::{fs, io};
use subprocess::{Exec, Redirection};

mod emoji;
mod parser;

enum ApplicationError {
    BaseDirNotFound,
    BaseDirCannotBeOpened(std::io::Error),
    CantCreateTargetDir(std::io::Error),
    CantDeleteTargetDir(std::io::Error),
    FailedCloneCommand(subprocess::PopenError),
    FailedCheckoutCommand(subprocess::PopenError),
    FailedGitOperation(),
    FailedParsingRepo(parser::ParseRepoError),
    FailedCaptureInput(std::io::Error),
    ArgumentParsingError(getopts::Fail),
    RemoteBranchNotFound(String),
    FailedRemoteBranchCheck(subprocess::PopenError),
}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ApplicationError::BaseDirNotFound => {
                write!(f, "The base directory on which to download the repositories was not found. Ensure you have set the $GC_DOWNLOAD_PATH or $GOPATH environment variable.")
            }
            ApplicationError::BaseDirCannotBeOpened(err) => {
                write!(f, "Base directory cannot be opened: {}", err)
            }
            ApplicationError::CantCreateTargetDir(err) => {
                write!(f, "Cannot create target directory: {}", err)
            }
            ApplicationError::CantDeleteTargetDir(err) => {
                write!(f, "Cannot delete target directory: {}", err)
            }
            ApplicationError::FailedCloneCommand(err) => {
                write!(f, "Failed to run the git clone command: {}", err)
            }
            ApplicationError::FailedCheckoutCommand(err) => {
                write!(f, "Failed to run the git checkout command: {}", err)
            }
            ApplicationError::FailedGitOperation() => {
                write!(f, "Failed to clone the repo.")
            }
            ApplicationError::FailedCaptureInput(err) => {
                write!(f, "Failed to capture prompt: {}", err)
            }
            ApplicationError::FailedParsingRepo(err) => {
                write!(f, "Failed to parse the repository URL: {}", err)
            }
            ApplicationError::ArgumentParsingError(err) => {
                write!(f, "Failed to parse arguments: {}", err)
            }
            ApplicationError::RemoteBranchNotFound(branch) => {
                write!(f, "Remote branch \"{}\" not found", branch)
            }
            ApplicationError::FailedRemoteBranchCheck(err) => {
                write!(f, "Failed to check remote branch: {}", err)
            }
        }
    }
}

impl From<parser::ParseRepoError> for ApplicationError {
    fn from(err: parser::ParseRepoError) -> Self {
        ApplicationError::FailedParsingRepo(err)
    }
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{} Error: {}", get_emoji("x", "\u{f071}"), err);
            std::process::exit(1);
        }
    }
}

/// Get an emoji by shortcode or fallback to a unicode code point or string
fn get_emoji(emoji_code: &str, fallback: &str) -> String {
    Emoji(
        emojis::get_by_shortcode(emoji_code).unwrap().as_str(),
        fallback,
    )
    .to_string()
}

fn run() -> Result<(), ApplicationError> {
    // Get the base directory
    let base_dir = env::var("GC_DOWNLOAD_PATH")
        .or_else(|_| env::var("GOPATH"))
        .map_err(|_| ApplicationError::BaseDirNotFound)?;
    let base_dir = format!("{}/src", base_dir);

    // Get the default user if one is set
    let default_user = env::var("GC_GITHUB_USERNAME")
        .or_else(|_| env::var("GITHUB_USERNAME"))
        .unwrap_or("".to_string());

    // Try opening the base directory
    fs::read_dir(&base_dir).map_err(ApplicationError::BaseDirCannotBeOpened)?;

    // Get the repository URL from the command line arguments
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt(
        "b",
        "branch",
        "set the branch to checkout after cloning",
        "BRANCH",
    );

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => return Err(ApplicationError::ArgumentParsingError(f)),
    };

    let repo_url = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        eprintln!("Usage: gc <repository-url> [-b <branch>]");
        return Ok(());
    };

    let branch = matches.opt_str("b");

    // Parse the repository URL
    let (host, team, project) = parser::repository(default_user, repo_url.to_string())?;
    let project_path = format!("{}/{}/{}/{}", base_dir, host, team, project);
    let clone_url = format!("git@{}:{}/{}.git", host, team, project);

    // Create the directory if it does not exist
    if !Path::new(&project_path).exists() {
        eprintln!(
            "{} Destination directory for {}/{} does not exist. Creating...",
            get_emoji("white_check_mark", "\u{ea83}"),
            team,
            project
        );
        fs::create_dir_all(&project_path).map_err(ApplicationError::CantCreateTargetDir)?;
    } else {
        eprintln!(
            "{} Destination directory for {}/{} already exists.",
            get_emoji("warning", "\u{eb32}"),
            team,
            project
        );
        eprintln!("Press <Enter> to confirm deletion or <Ctrl+C> to cancel...");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(ApplicationError::FailedCaptureInput)?;
        fs::remove_dir_all(&project_path).map_err(ApplicationError::CantDeleteTargetDir)?;
        fs::create_dir_all(&project_path).map_err(ApplicationError::CantCreateTargetDir)?;
    }

    // Run the git clone command
    eprintln!(
        "{} Cloning {}/{}...",
        get_emoji("inbox_tray", "\u{ebcc}"),
        team,
        project,
    );

    let exec = Exec::cmd("git")
        .args(&["clone", &clone_url, &project_path])
        .cwd(env::temp_dir())
        .stdout(Redirection::None)
        .stderr(Redirection::None)
        .capture()
        .map_err(ApplicationError::FailedCloneCommand)?;

    if !exec.success() {
        return Err(ApplicationError::FailedGitOperation());
    }

    eprintln!(
        "{} Successfully cloned {}/{} into {}",
        get_emoji("white_check_mark", "\u{f058}"),
        team,
        project,
        project_path
    );

    if let Some(branch) = branch {
        eprintln!(
            "{} Checking if remote branch \"{}\" exists...",
            get_emoji("mag", "\u{f52d}"),
            branch
        );

        // Check if the remote branch exists
        let exec = Exec::cmd("git")
            .args(&["ls-remote", "--heads", "origin", &branch])
            .cwd(&project_path)
            .stdout(Redirection::Pipe)
            .stderr(Redirection::None)
            .capture()
            .map_err(ApplicationError::FailedRemoteBranchCheck)?;

        if !exec.success() {
            return Err(ApplicationError::FailedGitOperation());
        }

        let output = String::from_utf8_lossy(&exec.stdout);
        if output.trim().is_empty() {
            return Err(ApplicationError::RemoteBranchNotFound(branch.clone()));
        }

        eprintln!("Remote branch \"{}\" found", branch);

        eprintln!(
            "{} Checking out branch \"{}\"...",
            get_emoji("twisted_rightwards_arrows", "\u{f5c4}"),
            branch
        );

        // Create and checkout a local branch that tracks the remote branch
        let exec = Exec::cmd("git")
            .args(&["checkout", "-b", &branch, &format!("origin/{}", branch)])
            .cwd(&project_path)
            .stdout(Redirection::None)
            .stderr(Redirection::None)
            .capture()
            .map_err(ApplicationError::FailedCheckoutCommand)?;

        if !exec.success() {
            return Err(ApplicationError::FailedGitOperation());
        }

        eprintln!(
            "{} Successfully checked out branch \"{}\"",
            get_emoji("white_check_mark", "\u{f5c4}"),
            branch
        );
    }

    println!("{}", project_path);
    Ok(())
}
