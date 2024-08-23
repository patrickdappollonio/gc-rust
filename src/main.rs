use getopts::Options;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::{env, fmt};
use std::{fs, io};
use subprocess::{Exec, Redirection};

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
            eprintln!("\u{f071} Error: {}", err);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), ApplicationError> {
    // Get the base directory
    let base_dir = env::var("GC_DOWNLOAD_PATH")
        .or_else(|_| env::var("GOPATH"))
        .map_err(|_| ApplicationError::BaseDirNotFound)?;
    let base_dir = format!("{}/src", base_dir);

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
    let (host, team, project) = parser::repository(repo_url.to_string())?;
    let project_path = format!("{}/{}/{}/{}", base_dir, host, team, project);
    let clone_url = format!("git@{}:{}/{}.git", host, team, project);

    // Create the directory if it does not exist
    if !Path::new(&project_path).exists() {
        eprintln!("\u{ea83} Destination directory does not exist. Creating...",);
        fs::create_dir_all(&project_path).map_err(ApplicationError::CantCreateTargetDir)?;
    } else {
        eprint!("\u{eb32} Destination directory already exists. Press <Enter> to confirm deletion or <Ctrl+C> to cancel...");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(ApplicationError::FailedCaptureInput)?;
        fs::remove_dir_all(&project_path).map_err(ApplicationError::CantDeleteTargetDir)?;
        fs::create_dir_all(&project_path).map_err(ApplicationError::CantCreateTargetDir)?;
    }

    // Run the git clone command
    eprintln!("\u{ebcc} Cloning {}/{}...", team, project);

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
        "\u{f058} Successfully cloned {}/{} into {}",
        team, project, project_path
    );

    if let Some(branch) = branch {
        eprintln!("\u{f5c4} Checking out branch {}...", branch);

        let exec = Exec::cmd("git")
            .args(&["checkout", &branch])
            .cwd(&project_path)
            .stdout(Redirection::None)
            .stderr(Redirection::None)
            .capture()
            .map_err(ApplicationError::FailedCheckoutCommand)?;

        if !exec.success() {
            return Err(ApplicationError::FailedGitOperation());
        }

        eprintln!("\u{f5c4} Successfully checked out branch {}", branch);
    }

    println!("{}", project_path);
    Ok(())
}
