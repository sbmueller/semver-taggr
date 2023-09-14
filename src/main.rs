use clap::Parser;
use git2::Repository;
use log::{error, info};
use std::path::PathBuf;

mod elements;
mod functions;
use functions::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Directory of git repository to tag. Defaults to current directory
    work_dir: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Force working on another branch than master or main
    #[arg(short, long)]
    force: bool,
}

fn main() {
    let cli = Cli::parse();
    initialize_logging(cli.debug);

    // Read repository location or set to working directory
    let work_dir = match cli.work_dir {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_dir().expect("Could not read current working directory."),
    };

    // Open git repository at location
    let repo = Repository::open(&work_dir).unwrap_or_else(|_| {
        panic!(
            "Could not open git repository at: {}",
            &work_dir.as_path().display()
        )
    });

    info!("Repository location: {}", &work_dir.as_path().display());

    if !cli.force && !on_master_branch(&repo) {
        error!("Master/main branch not checked out, aborting.");
        return;
    }

    let last_tag = find_latest_semver_tag(&repo).expect("Error with tags");

    let (tag_prefix, mut major, mut minor, mut patch, tag_suffix) = split_tag_semver(&last_tag)
        .unwrap_or_else(|| panic!("Version could not be found in tag: {}", last_tag));

    info!("Last tagged version: {}.{}.{}", major, minor, patch);

    let bump = prompt_bump_element();

    semver_bump(&mut major, &mut minor, &mut patch, &bump);
    let new_tag = format!("{}{}.{}.{}{}", tag_prefix, major, minor, patch, tag_suffix);

    create_new_tag(&repo, &new_tag).expect("Could not create new tag.");
}
