use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use reqwest::header::USER_AGENT;

fn unzip(zip_file_name: &str) -> String {
    let fname = std::path::Path::new(zip_file_name);
    let file = fs::File::open(&fname).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    let mut folder_name = "".to_string();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if i == 0 {
            // Sanity check that it's a folder
            assert!((*file.name()).ends_with('/'));

            folder_name = format!("{}", outpath.display());
            println!("{}", folder_name);
        }

        if (*file.name()).ends_with('/') {
            // println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            // println!(
            //     "File {} extracted to \"{}\" ({} bytes)",
            //     i,
            //     outpath.display(),
            //     file.size()
            // );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    folder_name
}

pub fn check_github_api(url: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    println!("Checking GitHub API");
    let github_repo_api_pulls_url = url;
    let user_agent = "GeckoEidechse/northstar-dev-testing-helper-tool";
    let client = reqwest::blocking::Client::new();
    let res = client
        .get(github_repo_api_pulls_url)
        .header(USER_AGENT, user_agent)
        .send()?
        .text()?;
    // println!("{:#?}", res);

    let json: serde_json::Value = serde_json::from_str(&res).expect("JSON was not well-formatted");
    println!("Done checking GitHub API");

    Ok(json)
}

fn get_mods_download_link(pr_number: i64, json_response: serde_json::Value) -> String {
    for elem in json_response.as_array().unwrap() {
        for val in elem.as_object().unwrap() {
            let (key, v) = val;

            if key == "number" && v.as_i64().unwrap() == pr_number {
                for val in elem.as_object().unwrap() {
                    let (key, v) = val;
                    if key == "head" {
                        let mut json_key_ref = "";
                        let mut json_key_fullname = "";
                        for val in v.as_object().unwrap() {
                            let (key, v) = val;

                            if key == "ref" {
                                // println!("{}", v);
                                json_key_ref = v.as_str().unwrap();
                            }
                            if key == "repo" {
                                for val in v.as_object().unwrap() {
                                    let (key, v) = val;
                                    if key == "full_name" {
                                        json_key_fullname = v.as_str().unwrap();
                                    }
                                }
                            }
                            // println!("{} {}", json_key_ref, json_key_fullname);
                        }
                        let download_url = format!(
                            "https://github.com/{}/archive/refs/heads/{}.zip",
                            json_key_fullname, json_key_ref
                        );
                        return download_url;
                        // break;
                    }
                }
            }
        }
    }
    "".to_string()
}

fn download_zip(download_url: String, location: String) {
    println!("Downloading file");
    let user_agent = "GeckoEidechse/northstar-dev-testing-helper-tool";
    let client = reqwest::blocking::Client::new();
    let mut resp = client
        .get(download_url)
        .header(USER_AGENT, user_agent)
        .send()
        .unwrap();
    let mut out = File::create(format!("{}/ns-dev-test-helper-temp-pr-files.zip", location))
        .expect("failed to create file");
    io::copy(&mut resp, &mut out).expect("failed to copy content");
    println!("Download done");
}

/// Recursively copies files from one directory to another
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn add_batch_file(game_install_path: &str) {
    let batch_path = format!("{}/r2ns-launch-mod-pr-version.bat", game_install_path);
    let path = Path::new(&batch_path);
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    // Write the string to `file`, returns `io::Result<()>`
    let batch_file_content =
        "NorthstarLauncher.exe -profile=R2Northstar-PR-test-managed-folder\r\n";

    match file.write_all(batch_file_content.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => println!("successfully wrote to {}", display),
    }
}

pub fn apply_mods_pr(
    pr_number: i64,
    game_install_path: &str,
    json_response: serde_json::Value,
) -> bool {
    println!("{}", pr_number);
    println!("{}", game_install_path);
    let is_correct_game_path =
        std::path::Path::new(&format!("{}/Titanfall2.exe", game_install_path)).exists();
    println!("Titanfall2.exe exists in path? {}", is_correct_game_path);

    // Exit early if wrong game path
    if !is_correct_game_path {
        println!("Incorrect path");
        return false; // Return false to signal error, should use enum or option in the future
    }

    let download_url = get_mods_download_link(pr_number, json_response);

    println!("{}", download_url);

    download_zip(download_url, ".".to_string());

    let zip_extract_folder_name = unzip("ns-dev-test-helper-temp-pr-files.zip");

    println!("Zip extract done");

    println!("Deleting temp zip download folder");

    fs::remove_file("ns-dev-test-helper-temp-pr-files.zip").unwrap();

    // TODO: delete downloaded zip folder again here

    // Delete previously managed folder
    if std::fs::remove_dir_all(format!(
        "{}/R2Northstar-PR-test-managed-folder",
        game_install_path
    ))
    .is_err()
    {
        if std::path::Path::new(&format!(
            "{}/R2Northstar-PR-test-managed-folder",
            game_install_path
        ))
        .exists()
        {
            println!("Failed removing previous dir"); // TODO check if exists and only panic if no exists
        } else {
            println!("Failed removing folder that doesn't exist. Probably cause first run");
        }
    };

    println!("Copying files to Titanfall2 install");

    // Copy downloaded folder to game install folder
    copy_dir_all(
        zip_extract_folder_name.clone(),
        format!(
            "{}/R2Northstar-PR-test-managed-folder/mods",
            game_install_path
        ),
    )
    .unwrap();

    println!("Deleting old unzipped folder");

    // Delete old copy
    std::fs::remove_dir_all(zip_extract_folder_name).unwrap();

    println!("Adding batch file to 1-click-run PR");

    add_batch_file(game_install_path);

    println!("All done :D");

    true
}
