use std::fs;
use std::path::Path;
use std::process::Command;

use dialoguer::{Confirm, MultiSelect};
use glob::{glob_with, MatchOptions};

#[macro_use]
extern crate clap;
extern crate dirs;

#[derive(Debug)]
struct Application {
    path: String,
    bundle_id: String,
}

fn get_bundle_id(path: &str) -> Option<String> {
    // mdls -name kMDItemCFBundleIdentifier -r /Applications/AppName.app
    let k_mditem_cfbundle_identifier = Command::new("mdls")
        .arg("-name")
        .arg("kMDItemCFBundleIdentifier")
        .arg("-r")
        .arg(path)
        .output()
        .expect("Failed to execute mdls")
        .stdout;

    let bundle_id = String::from_utf8(k_mditem_cfbundle_identifier).unwrap_or("".to_string());
    Some(bundle_id)
}

fn main() {
    // Parse args
    let matches = clap_app!(appzap =>
        (version: "0.1.0")
        (author: "Jordan Bertasso")
        (about: "A command line tool to uninstall MacOS apps and their related files")
        (@arg APPLICATION: +required +takes_value "The path of the application to uninstall")
        (@arg deep: -d --deep "Whether to recursive glob with ** or not")
    )
    .get_matches();

    // Get application path
    let app_path = matches
        .value_of("APPLICATION")
        .expect("Application path not provided");

    // Initialise app struct
    let app = Application {
        path: app_path.to_owned(),
        bundle_id: get_bundle_id(&app_path).unwrap_or(String::default()),
    };

    // println!("{:?}", app);

    // Initialise locations to check for files
    let home_dir = dirs::home_dir().expect("No home dir");
    let library = home_dir.join("Library");
    let prefs = library.join("Preferences");
    let home_apps = home_dir.join("Applications");
    let apps = Path::new("/Applications");
    let system_apps = Path::new("/System/Applications");
    let app_support;
    let caches;

    if matches.is_present("deep") {
        app_support = library.join("Application Support/**");
        caches = library.join("Caches/**");
    } else {
        app_support = library.join("Application Support");
        caches = library.join("Caches");
    }

    let mut locs = vec![
        app_support,
        caches,
        prefs,
        home_apps,
        apps.to_path_buf(),
        system_apps.to_path_buf(),
    ];

    // Add the parent directory if it's not included in our search path
    let parent_dir = Path::new(&app.path)
        .parent()
        .expect("No parent")
        .to_path_buf();

    if !locs.contains(&parent_dir) {
        locs.push(parent_dir);
    }

    // What we should look for in the above locations
    let ids = vec![
        app.bundle_id.as_str(),
        Path::new(&app.path)
            .file_name()
            .expect("No file name")
            .to_str()
            .expect("Could not convert file name to string")
            .split(".")
            .next()
            .expect("No file name"),
    ];

    // Set case insensitive
    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let mut found = vec![];
    // Glob for matches in those locations for the app name and bundle ID and print the matches
    for loc in locs {
        for id in &ids {
            let joined_path = format!(
                "{}/{}*",
                loc.to_str().expect("Could not convert to string"),
                id
            );

            for file in glob_with(&joined_path, options).expect("Failed to read glob pattern") {
                if let Ok(path) = file {
                    // println!("{}", path.display());
                    found.push(path.display().to_string());
                }
            }
        }
    }

    if found.is_empty() {
        println!("No files found");
        return;
    }

    let chosen: Vec<usize> = MultiSelect::new()
        .with_prompt("Files to delete")
        .items(&found)
        .interact()
        .expect("Could not get selected files");

    // println!("found: {:?}", found);
    // println!("chosen: {:?}", chosen);

    let to_delete = {
        let mut res = vec![];
        for i in chosen {
            res.push(found[i].as_str());
        }
        res
    };

    println!("to_delete: {:?}", to_delete);

    let confirmed = Confirm::new()
        .with_prompt("Are you sure?")
        .interact()
        .expect("Failed to get confirmation");
    println!("confirmed: {}", confirmed);

    if confirmed {
        for file in to_delete {
            let p = Path::new(file);
            // fs::rename(p, home_dir.join(".Trash")).expect("Unable to move file to Trash");
            // fs::copy(p, home_dir.join(".Trash")).expect("Unable to move file to Trash");
            println!("Removing {:?}", p);
            match fs::remove_dir_all(p) {
                Err(_) => fs::remove_file(p).expect("Unable to remove files"),
                _ => continue,
            }
        }
    }
}
