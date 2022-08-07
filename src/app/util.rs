use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use reqwest::header::USER_AGENT;

use anyhow::anyhow;

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

fn unzip_launcher_zip(zip_file_name: &str) -> String {
    let outfolder_name = "ns-dev-test-helper-temp-pr-files";
    let fname = std::path::Path::new(zip_file_name);
    let file = fs::File::open(&fname).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    fs::create_dir_all(outfolder_name).unwrap();

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

        // Only extract two hardcoded files
        if *file.name() == *"NorthstarLauncher.exe" || *file.name() == *"Northstar.dll" {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile =
                fs::File::create(format!("{}/{}", outfolder_name, outpath.display())).unwrap();
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
    outfolder_name.to_string()
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

fn get_mods_download_link(
    pr_number: i64,
    json_response: serde_json::Value,
) -> Result<String, anyhow::Error> {
    // {pr object} -> number == pr_number
    //             -> head -> ref
    //                     -> repo -> full_name
    for elem in json_response.as_array().unwrap() {
        // Early return if PR number is not the right one
        if elem.get("number").and_then(|value| value.as_i64()).unwrap() != pr_number {
            continue;
        }

        // Get branch name
        let json_key_ref = elem
            .get("head")
            .and_then(|value| value.get("ref"))
            .and_then(|value| value.as_str())
            .unwrap();

        // Get repo name
        let json_key_fullname = elem
            .get("head")
            .and_then(|value| value.get("repo"))
            .and_then(|value| value.get("full_name"))
            .and_then(|value| value.as_str())
            .unwrap();

        // Use repo and branch name to get download link
        let download_url = format!(
            "https://github.com/{}/archive/refs/heads/{}.zip",
            json_key_fullname, json_key_ref
        );
        return Ok(download_url);
    }
    return Err(anyhow!(
        "Couldn't grab download link for PR \"{}\"",
        pr_number
    ));
}

fn get_launcher_download_link(
    pr_number: i64,
    json_response: serde_json::Value,
) -> Result<String, anyhow::Error> {
    // Crossreference with runs API
    let runs_json_response = match check_github_api(
        "https://api.github.com/repos/R2Northstar/NorthstarLauncher/actions/runs",
    ) {
        Ok(result) => result,
        Err(err) => return Err(anyhow!(format!("{}", err))),
    };

    // Get top commit SHA
    for elem in json_response.as_array().unwrap() {
        // Early return if PR number is not the right one
        if elem.get("number").and_then(|value| value.as_i64()).unwrap() != pr_number {
            continue;
        }

        // Get merge commit sha as short version is part of filename
        let json_key_merge_commit_sha = elem
            .get("merge_commit_sha")
            .and_then(|value| value.as_str())
            .unwrap();

        // Get head commit sha of PR
        let json_key_sha = elem
            .get("head")
            .and_then(|value| value.get("sha"))
            .and_then(|value| value.as_str())
            .unwrap();

        // Cross-reference PR head commit sha against workflow runs
        let v = runs_json_response.get("workflow_runs").unwrap();
        for elem in v.as_array().unwrap() {
            // Get commit show on which run was performed
            let json_key_head_sha = elem
                .get("head_sha")
                .and_then(|value| value.as_str())
                .unwrap();

            // Get run ID
            let json_key_id = elem.get("id").and_then(|value| value.as_i64()).unwrap();

            // If head commit sha of run and PR match, grab CI output
            if json_key_head_sha == json_key_sha {
                dbg!(json_key_id);
                dbg!(json_key_sha);
                dbg!(json_key_merge_commit_sha);

                // Check artifacts
                println!("Checking: https://api.github.com/repos/R2Northstar/NorthstarLauncher/actions/runs/{}/artifacts", json_key_id);
                let artifacts_json_response = check_github_api(&format!("https://api.github.com/repos/R2Northstar/NorthstarLauncher/actions/runs/{}/artifacts", json_key_id))
                    .expect("Failed request");

                // Iterate over artifacts
                for elem in artifacts_json_response
                    .get("artifacts")
                    .and_then(|value| value.as_array())
                    .unwrap()
                {
                    // Make sure run is from PR head commit
                    let current_run_json_key_sha = elem
                        .get("workflow_run")
                        .and_then(|value| value.get("head_sha"))
                        .and_then(|value| value.as_str())
                        .unwrap();
                    if current_run_json_key_sha == json_key_head_sha {
                        let artifact_id_json_key_sha =
                            elem.get("id").and_then(|value| value.as_i64()).unwrap();

                        // Download artifact
                        return Ok(format!("https://nightly.link/R2Northstar/NorthstarLauncher/actions/artifacts/{}.zip", artifact_id_json_key_sha));
                    }
                }
            }
        }
    }
    return Err(anyhow!(
        "Couldn't grab download link for PR \"{}\"",
        pr_number
    ));
}

fn download_zip(download_url: String, location: String) -> Result<(), anyhow::Error> {
    println!("Downloading file");
    let user_agent = "GeckoEidechse/northstar-dev-testing-helper-tool";
    let client = reqwest::blocking::Client::new();
    let mut resp = client
        .get(download_url)
        .header(USER_AGENT, user_agent)
        .send()
        .unwrap();

    // Error out earlier if non-successful response
    if !resp.status().is_success() {
        println!("Status: {}", resp.status());
        // Return error cause wrong game path
        return Err(anyhow!(
            "Couldn't download zip. Received error code \"{}\"",
            resp.status()
        ));
    }

    let mut out = File::create(format!("{}/ns-dev-test-helper-temp-pr-files.zip", location))
        .expect("failed to create file");
    io::copy(&mut resp, &mut out).expect("failed to copy content");
    println!("Download done");
    Ok(())
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

/// Checks whether the provided path is a valid Titanfall2 gamepath by checking against a certain set of criteria
fn check_game_path(game_install_path: &str) -> Result<(), anyhow::Error> {
    let is_correct_game_path =
        std::path::Path::new(&format!("{}/Titanfall2.exe", game_install_path)).exists();
    println!("Titanfall2.exe exists in path? {}", is_correct_game_path);

    // Exit early if wrong game path
    if !is_correct_game_path {
        return Err(anyhow!("Incorrect game path \"{}\"", game_install_path)); // Return error cause wrong game path
    }
    Ok(())
}

pub fn apply_launcher_pr(
    pr_number: i64,
    game_install_path: &str,
    json_response: serde_json::Value,
) -> Result<(), anyhow::Error> {
    println!("{}", pr_number);
    println!("{}", game_install_path);

    // Exit early if wrong game path
    check_game_path(game_install_path)?;

    // get download link
    let download_url = get_launcher_download_link(pr_number, json_response)?;

    println!("{}", download_url);

    // download
    download_zip(download_url, ".".to_string())?;

    // extract
    let zip_extract_folder_name = unzip_launcher_zip("ns-dev-test-helper-temp-pr-files.zip");

    println!("Zip extract done");

    println!("Deleting temp zip download folder");

    fs::remove_file("ns-dev-test-helper-temp-pr-files.zip").unwrap();

    // Copy downloaded folder to game install folder
    copy_dir_all(zip_extract_folder_name.clone(), game_install_path).unwrap();

    println!("Deleting old unzipped folder");

    // Delete old copy
    std::fs::remove_dir_all(zip_extract_folder_name).unwrap();

    println!("All done :D");

    Ok(())
}

pub fn apply_mods_pr(
    pr_number: i64,
    game_install_path: &str,
    json_response: serde_json::Value,
) -> Result<(), anyhow::Error> {
    println!("{}", pr_number);
    println!("{}", game_install_path);

    // Exit early if wrong game path
    check_game_path(game_install_path)?;

    let download_url = get_mods_download_link(pr_number, json_response)?;

    println!("{}", download_url);

    download_zip(download_url, ".".to_string())?;

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

    Ok(())
}
