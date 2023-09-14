use git2::{ObjectType, Repository, Signature};
use inquire::{Confirm, Select};
use log::{debug, info, LevelFilter};

use regex::Regex;
use simple_logger::SimpleLogger;

use crate::elements::Type;

const SEMVER_REGEX: &str = r"(.*)(\d+)\.(\d+)\.(\d+)(.*)";

/// Find the latest tag containing a semantic version in the given repository that is reachable
/// from the currently checked out commit.
///
/// * `repo`: Repository to look for tag
pub fn find_latest_semver_tag(repo: &Repository) -> Result<String, git2::Error> {
    // Create a DescribeOptions struct
    let mut opts = git2::DescribeOptions::new();
    let mut format_opts = git2::DescribeFormatOptions::new();
    opts.describe_tags(); // Use tags as references
    opts.pattern("*[0-9]*.[0-9]*.[0-9]*");
    format_opts.abbreviated_size(0);
    opts.show_commit_oid_as_fallback(false); // Do not show commit id if no tag is found

    // Get the most recent tag name
    let tag_name = repo.describe(&opts)?.format(Some(&format_opts))?;

    debug!("The most recent tag is: {}", tag_name);
    Ok(tag_name)
}

/// Returns true if provided repository has master/main branch checked out, false otherwise.
///
/// * `repo`: Repository to check
pub fn on_master_branch(repo: &Repository) -> bool {
    if let Ok(head) = repo.head() {
        // Get the shorthand reference name (e.g., "refs/heads/master")
        if let Some(branch_name) = head.shorthand() {
            // Compare the branch name to "master"
            if branch_name == "master" || branch_name == "main" {
                return true;
            }
        }
    }
    false
}

/// Extract the semantic version parts major, minor and patch from a string as well as their pre-
/// and suffixes.
/// It is assumed the string contains exactly one section with three consecutive numbers, separated
/// by a period (.).
///
/// # Example
/// ```
/// let a = split_tag_semver("abcd-1.2.3-efgh");
/// assert!(a == ("abcd", 1, 2, 3, "efgh"));
/// ```
pub fn split_tag_semver(tag: &str) -> Option<(String, u32, u32, u32, String)> {
    // Safety: Regex is verified to be valid
    let re = Regex::new(SEMVER_REGEX).unwrap();

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

/// Set logging to the desired level.
///
/// * `debug`: Debug level
pub fn initialize_logging(debug: u8) {
    SimpleLogger::new().with_colors(true).init().unwrap();
    match debug {
        1 => log::set_max_level(LevelFilter::Debug),
        2 => log::set_max_level(LevelFilter::Trace),
        _ => log::set_max_level(LevelFilter::Info),
    }
}

/// Create a new tag if confirmed by a prompt on the HEAD of the provided repository.
///
/// * `repo`: Repository to tag
/// * `tag_name`: Name of the tag to create
pub fn create_new_tag(repo: &Repository, tag_name: &str) -> Result<bool, git2::Error> {
    // Confirm tag creation
    let ans = Confirm::new(&format!("Create new tag {}?", tag_name))
        .with_default(true)
        .prompt()
        .unwrap();

    if !ans {
        info!("Aborting.");
        return Ok(false);
    }

    let tag_message = "Tag created by taggr";
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

    info!("Annotated tag created: {} on {}", tag_name, tag_oid);

    Ok(true)
}

/// Prompt the user which semantic version element shall be increased (Major, Minor, Patch).
/// Returns the element.
pub fn prompt_bump_element() -> Type {
    let options: Vec<Type> = vec![Type::Major, Type::Minor, Type::Patch];

    Select::new("Which version to bump?", options)
        .prompt()
        .unwrap()
}

/// Bump the version segments according to the selected bump.
///
/// # Arguments
/// * `major`: Mutable reference to major version
/// * `minor`: Mutable reference to minor version
/// * `patch`: Mutable reference to patch version
/// * `bump`: Specification of which segment to bump
pub fn semver_bump(major: &mut u32, minor: &mut u32, patch: &mut u32, bump: &Type) {
    debug!("Bumping {}.", bump);
    match bump {
        Type::Major => {
            *major += 1;
            *minor = 0;

            *patch = 0;
        }
        Type::Minor => {
            *minor += 1;
            *patch = 0;
        }
        Type::Patch => *patch += 1,
    }
}
