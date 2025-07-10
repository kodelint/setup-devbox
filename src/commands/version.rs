// Let's bring in the tools we'll need!
// We're using custom logging functions (log_error, log_info, log_warn)
// Details about the logging macros can found in `src/logger.rs`
// that are likely set up elsewhere in our project to give us nice, colorful messages.
use crate::{log_error, log_info, log_warn};
// This 'colored' crate helps us make our terminal output pop with different colors!
use colored::Colorize;
// Serde is super handy for turning structured data (like JSON or TOML) into Rust structs,
// and 'Deserialize' is the part that handles reading that data in.
use serde::Deserialize;
// We'll need 'std::fs' to read files from our computer, like our Cargo.toml.
use std::fs;
// 'std::io' is our go-to for input/output operations, and it helps us manage file errors.
use std::io;
// 'ureq' is a neat, simple HTTP client that makes it easy to grab data from the internet.
use ureq;

// These are just some placeholders for the GitHub repository we're checking.
// Remember to swap 'Random' with the actual owner and repo name when you use this!
const REPO_OWNER: &str = "kodelint";
const REPO_NAME: &str = "setup-devbox";

/// Imagine our `Cargo.toml` file. This struct helps us map its structure directly into Rust!
/// We're telling Serde: "Hey, expect a section called `[package]` here."
#[derive(Deserialize)]
struct CargoToml {
    package: Package, // This will hold all the goodies from our `[package]` section.
}

/// Now, let's dive into that `[package]` section.
/// This struct specifically looks for the `version` inside it.
#[derive(Deserialize)]
struct Package {
    version: String, // This is where we'll pluck out our local software's version number.
}

/// This little function's job is to go peek inside your local `Cargo.toml` file
/// and fetch the version number of *this* project.
///
/// It's designed to either hand you back the version (as a `String`) or tell you if something went wrong.
fn get_local_version() -> io::Result<String> {
    // First, we try to read the whole `Cargo.toml` file into a string.
    // If it can't find the file or read it for any reason, the `?` will gracefully
    // pass that error along.
    let cargo_toml = fs::read_to_string("Cargo.toml")?;

    // Next, we attempt to turn that raw TOML text into our structured `CargoToml` Rust type.
    // If parsing fails (maybe the TOML is malformed?), we'll catch that error
    // and turn it into an `io::Error` so all our errors speak the same language.
    let cargo: CargoToml = toml::from_str(&cargo_toml).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other, // This is a general "something else went wrong" kind of error.
            format!("Oops! Couldn't parse Cargo.toml: {}", e), // A friendly message about the parsing failure.
        )
    })?; // Again, `?` ensures any errors here get handled by the caller.

    // If all goes well, we've got our version! Let's send it back.
    Ok(cargo.package.version)
}

/// When we ask GitHub for release info, it sends back some JSON.
/// This struct helps us pick out just the `tag_name` from that JSON, which is usually the version.
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String, // This is what we're really after: the version tag from GitHub.
}

/// This function reaches out to GitHub to find the latest official release version
/// for our specified repository.
///
/// It aims to return the latest version tag or explain what went wrong.
fn get_latest_github_release() -> Result<String, Box<dyn std::error::Error>> {
    // We build the specific URL we need to hit on GitHub's API.
    // It's like asking: "Hey GitHub, what's the latest release for [REPO_OWNER]/[REPO_NAME]?"
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );

    // We set up our HTTP client ('ureq'). It's good practice to tell the server who's asking,
    // so we set a 'User-Agent' header.
    let agent = ureq::AgentBuilder::new()
        .user_agent("setup-devbox-version-checker") // This identifies our application making the request.
        .build();

    // Now, let's actually send the request! If anything goes wrong during the network call,
    // the `?` will catch it and propagate the error.
    let response = agent.get(&url).call()?;

    // A quick check to make sure GitHub is actually sending us JSON,
    // not something else unexpected.
    if !response.has("content-type") // Does it even have a content-type header?
        || !response
            .header("content-type") // If it does, let's grab it.
            .unwrap() // We can safely unwrap here because we just checked if it exists.
            .contains("application/json")
    // And does it say "application/json"?
    {
        // If not, that's an issue! We'll return a custom error message.
        return Err("GitHub sent something unexpected, not JSON.".into());
    }

    // Alright, if we got valid JSON, let's try to parse it into our `GitHubRelease` struct.
    // If the JSON structure isn't what we expect, this will also turn into an error.
    let release: GitHubRelease = response.into_json()?;

    // Success! Here's the latest version tag from GitHub.
    Ok(release.tag_name)
}

/// This is the main orchestrator of our version checking!
/// It brings everything together: getting your local version, fetching the latest from GitHub,
/// and then telling you if you're up to date or need an upgrade.
pub fn run() {
    // Let's kick things off by checking what version we're running locally.
    log_info!("Checking your local version...");

    // We'll try to get the local version, and then decide what to do based on the result.
    match get_local_version() {
        // Hooray, we got the local version!
        Ok(local_version) => {
            log_info!("Found local version: {}", local_version);

            // Now, let's see what the world (or rather, GitHub) has to offer.
            log_info!("Checking for the latest GitHub release...");
            match get_latest_github_release() {
                // Awesome, we fetched the latest release from GitHub!
                Ok(latest_version) => {
                    log_info!("Latest GitHub release: {}", latest_version);

                    // Time to compare! We'll trim any extra spaces just to be safe.
                    if latest_version.trim() != local_version.trim() {
                        // Uh oh, looks like there's a newer version out there!
                        log_warn!("A newer version is available! Time to upgrade?");
                    } else {
                        // Good news! You're already on the cutting edge.
                        log_info!("You're already using the latest version. Great job!");
                    }
                }
                // Darn, couldn't get the latest release from GitHub.
                Err(e) => {
                    log_error!("Failed to fetch the latest release from GitHub: {}", e);
                }
            }
        }
        // Drat! We couldn't even figure out the local version.
        Err(e) => {
            log_error!("Couldn't read your local Cargo.toml version: {}", e);
        }
    }
}
