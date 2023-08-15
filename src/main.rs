use clap::Parser;
use git2::{ObjectType, Repository, Signature};
use inquire::{Confirm, Select};
use log::{debug, info, warn, LevelFilter};
use regex::Regex;
use simple_logger::SimpleLogger;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Directory of git repository to tag. Defaults to current directory
    work_dir: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Bump {Major, Minor, Patch}. When specified, command line will not ask it again.
    #[arg(long)]
    mmp: Option<String>,

    /// Do not perform any writing actions
    #[arg(long)]
    dry: bool,

    /// Skip Jenkins trigger
    #[arg(long = "no-jt")]
    no_jt: bool,

    /// Confirm all decision questions with "yes" (skipping confirmation of tag creation and push)
    #[arg(short)]
    yes: bool,
}

/// Determine the last tag of a repository. Returns an Option of the last tag as String.
fn get_last_tag(repo: &Repository) -> Option<String> {
    repo.tag_names(None)
        .unwrap()
        .iter()
        .last()
        .map_or(None, |x| Some(x.unwrap().to_owned()))
}

/// Extract the semantic version parts major, minor and patch from a string.
/// It is assumed the string contains exactly one section with three consecutive numbers, separated
/// by a period (.).
fn split_tag_semver(tag: &str) -> Option<(String, u32, u32, u32, String)> {
    // Safety: Regex is verified to be valid
    let re = Regex::new(r"(.*)(\d+)\.(\d+)\.(\d+)(.*)").unwrap();

    if let Some(captures) = re.captures(tag) {
        let tag_prefix = captures.get(1).unwrap().as_str();
        let major = captures.get(2).unwrap().as_str();
        let minor = captures.get(3).unwrap().as_str();
        let patch = captures.get(4).unwrap().as_str();
        let tag_suffix = captures.get(5).unwrap().as_str();

        debug!("Matched the following tag parts:");
        debug!("Prefix: {}", tag_prefix);
        debug!("Major: {}", major);
        debug!("Minor: {}", minor);
        debug!("Patch: {}", patch);
        debug!("Suffix: {}", tag_suffix);

        Some((
            tag_prefix.to_owned(),
            // Safety: Regex is defined to match numbers, so parsing to numbers must always succeed
            major.parse::<u32>().unwrap(),
            minor.parse::<u32>().unwrap(),
            patch.parse::<u32>().unwrap(),
            tag_suffix.to_owned(),
        ))
    } else {
        None
    }
}

fn set_log_level(debug: u8) {
    SimpleLogger::new().with_colors(true).init().unwrap();
    match debug {
        1 => log::set_max_level(LevelFilter::Debug),
        2 => log::set_max_level(LevelFilter::Trace),
        _ => log::set_max_level(LevelFilter::Info),
    }
}

fn create_new_tag(repo: &Repository, tag_name: &str) -> Result<(), git2::Error> {
    let tag_message = "Tag created by taggr-rs";
    // Get the HEAD reference
    let head = repo.head()?;
    let head_commit = head.peel(ObjectType::Commit)?;

    // Read user information from Git configuration
    let config = repo.config()?;
    let user_name = config.get_string("user.name")?;
    let user_email = config.get_string("user.email")?;

    // Create the annotated tag
    let user_signature = Signature::now(&user_name, &user_email)?;
    let tag_oid = repo.tag(tag_name, &head_commit, &user_signature, tag_message, false)?;

    info!("Annotated tag created: {}", tag_oid);

    Ok(())
}

/// Bump the version segments according to the selected bump.
///
/// # Arguments
/// * `major`: Mutable reference to major version
/// * `minor`: Mutable reference to minor version
/// * `patch`: Mutable reference to patch version
/// * `bump`: Specification of which segment to bump
///
/// # Errors
/// * If `bump` is neither "Major", "Minor" or "Patch", returns an Err.
fn semver_bump(major: &mut u32, minor: &mut u32, patch: &mut u32, bump: &str) -> Result<(), ()> {
    debug!("Bumping {}.", bump);
    match bump {
        "Major" => {
            *major += 1;
            *minor = 0;

            *patch = 0;
            Ok(())
        }
        "Minor" => {
            *minor += 1;
            *patch = 0;
            Ok(())
        }
        "Patch" => {
            *patch += 1;
            Ok(())
        }
        _ => Err(()),
    }
}

fn main() {
    let cli = Cli::parse();

    set_log_level(cli.debug);

    // Read repository location or set to working directory
    let work_dir = match cli.work_dir {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_dir().expect("Could not read current working directory."),
    };

    // Open git repository at location
    let repo = Repository::open(&work_dir).expect(&format!(
        "Could not open git repository at: {}",
        &work_dir.as_path().display()
    ));

    let last_tag = get_last_tag(&repo).expect(&format!(
        "No tag found in repository: {}",
        &work_dir.as_path().display()
    ));

    let (tag_prefix, mut major, mut minor, mut patch, tag_suffix) = split_tag_semver(&last_tag)
        .expect(&format!("Version could not be found in tag: {}", last_tag));

    println!("Last tagged version: {}.{}.{}", major, minor, patch);

    let options: Vec<&str> = vec!["Major", "Minor", "Patch"];

    let bump = Select::new("Which version to bump?", options)
        .prompt()
        .unwrap();

    // Safety: `bump` can only have a valid value at this point
    semver_bump(&mut major, &mut minor, &mut patch, &bump).unwrap();
    let new_tag = format!("{}{}.{}.{}{}", tag_prefix, major, minor, patch, tag_suffix);
    let ans = Confirm::new(&format!("Create new tag {}?", new_tag))
        .with_default(false)
        .prompt()
        .unwrap();

    if !ans {
        info!("Aborting.");
        return;
    }

    create_new_tag(&repo, &new_tag).expect("Could not create new tag.");
}
