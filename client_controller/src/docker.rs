use crate::config::{ControllerConfig, MatchRequest};
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub fn run_match(run_type: &str, config: &ControllerConfig, request: &MatchRequest) {
    let match_directory = format!("{}/match", config.logs_directory);
    let bot1_directory;
    let bot2_directory;

    if run_type == "aiarena" {
        // Bots are in separate directories on AI Arena
        bot1_directory = format!("{}/bot1/{}", config.bots_directory, request.bot1_name);
        bot2_directory = format!("{}/bot2/{}", config.bots_directory, request.bot2_name);
    } else {
        bot1_directory = format!("{}/{}", config.bots_directory, request.bot1_name);
        bot2_directory = format!("{}/{}", config.bots_directory, request.bot2_name);
    }

    // Identify the bot controllers
    let bot1_controller = if request.bot1_base.is_empty() {
        &config.bot_controller
    } else {
        &request.bot1_base
    };
    let bot2_controller = if request.bot2_base.is_empty() {
        &config.bot_controller
    } else {
        &request.bot2_base
    };

    // Prepare the template to schedule a match
    let template = include_str!("../templates/docker-compose.yaml");
    let template = template.replace("PLACEHOLDER_RUN_TYPE", run_type);
    let template = template.replace("PLACEHOLDER_VERSION", &config.version);
    let template = template.replace("PLACEHOLDER_API_URL", &config.api_url);
    let template = template.replace("PLACEHOLDER_GAME_CONTROLLER", &config.game_controller);
    let template = template.replace("PLACEHOLDER_BOTS_DIRECTORY", &config.bots_directory);
    let template = template.replace("PLACEHOLDER_BOT1_ID", &request.bot1_id);
    let template = template.replace("PLACEHOLDER_BOT1_NAME", &request.bot1_name);
    let template = template.replace("PLACEHOLDER_BOT1_CONTROLLER", bot1_controller);
    let template = template.replace("PLACEHOLDER_BOT1_DIRECTORY", &bot1_directory);
    let template = template.replace("PLACEHOLDER_BOT2_ID", &request.bot2_id);
    let template = template.replace("PLACEHOLDER_BOT2_NAME", &request.bot2_name);
    let template = template.replace("PLACEHOLDER_BOT2_CONTROLLER", bot2_controller);
    let template = template.replace("PLACEHOLDER_BOT2_DIRECTORY", &bot2_directory);
    let template = template.replace("PLACEHOLDER_GAMESETS_DIRECTORY", &config.gamesets_directory);
    let template = template.replace("PLACEHOLDER_LOGS_DIRECTORY", &config.logs_directory);
    let template = template.replace("PLACEHOLDER_MATCH_DIRECTORY", &match_directory);

    let mut compose_file = File::create("target/docker-compose.yaml")
        .unwrap_or_else(|e| panic!("Could not create docker-compose.yaml file: {e:?}"));
    compose_file.write_all(template.as_bytes())
        .unwrap_or_else(|e| panic!("Could not write to docker-compose.yaml file: {e:?}"));

    println!("\nDocker compose:\n{}", template);

    let status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg("target/docker-compose.yaml")
        .arg("up")
        .arg("-d")
        .arg("--force-recreate")
        .status()
        .expect("Failed to start docker compose");

    if !status.success() {
        eprintln!("Failed to start docker compose");
        std::process::exit(1);
    }

    let mut logs_process = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg("target/docker-compose.yaml")
        .arg("logs")
        .arg("-f")
        .spawn()
        .expect("Unable to stream docker compose logs");

    println!("Waiting for match_controller to exit...");
    loop {
        let output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg("target/docker-compose.yaml")
            .arg("ps")
            .arg("--format")
            .arg("json")
            .arg("match_controller")
            .output()
            .expect("Failed to check match_controller status");

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            // If output is empty or container is not running, it has exited
            if json_str.trim().is_empty() || !json_str.contains("\"State\":\"running\"") {
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    // Get exit code of match_controller
    let output = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg("target/docker-compose.yaml")
        .arg("ps")
        .arg("--all")
        .arg("--format")
        .arg("json")
        .arg("match_controller")
        .output()
        .expect("Failed to get match_controller status");

    let exit_code = if output.status.success() {
        let json_str = String::from_utf8_lossy(&output.stdout);
        // Extract exit code from JSON (e.g., "ExitCode":0)
        json_str
            .split("\"ExitCode\":")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(1)
    } else {
        1
    };

    println!("Match controller exited with code: {}", exit_code);

    // Stop logs process
    logs_process.kill().ok();
    logs_process.wait().ok();

    // Stop docker compose
    let status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg("target/docker-compose.yaml")
        .arg("down")
        .arg("--timeout=0")
        .status()
        .expect("Failed to stop docker compose");

    if !status.success() {
        eprintln!("Failed to stop docker compose");
    }

    if exit_code != 0 {
        eprintln!("Match controller failed with exit code: {}", exit_code);
        std::process::exit(exit_code);
    }
}
