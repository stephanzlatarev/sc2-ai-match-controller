use crate::config::{ControllerConfig, MatchRequest};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn run_match(run_type: &str, config: &ControllerConfig, request: &MatchRequest) {
    let match_directory = format!("{}/match", config.logs_directory);
    let (bot1_directory, bot2_directory) = select_bot_directories(run_type, config, request);
    let (bot1_controller, bot2_controller) = select_bot_controllers(config, request);
    let (bot1_command, bot2_command) = select_bot_commands(config, request);

    // Prepare the template to schedule a match
    let template = include_str!("../templates/docker-compose.yaml");
    let template = template.replace("PLACEHOLDER_RUN_TYPE", run_type);
    let template = template.replace("PLACEHOLDER_VERSION", &config.version);
    let template = template.replace("PLACEHOLDER_API_URL", &config.api_url);
    let template = template.replace("PLACEHOLDER_GAME_CONTROLLER", &config.game_controller);
    let template = template.replace("PLACEHOLDER_BOTS_DIRECTORY", &config.bots_directory);
    let template = template.replace("PLACEHOLDER_BOT1_ID", &request.bot1_id);
    let template = template.replace("PLACEHOLDER_BOT1_NAME", &request.bot1_name);
    let template = template.replace("PLACEHOLDER_BOT1_CONTROLLER", &bot1_controller);
    let template = template.replace("PLACEHOLDER_BOT1_COMMAND", &bot1_command);
    let template = template.replace("PLACEHOLDER_BOT1_DIRECTORY", &bot1_directory);
    let template = template.replace("PLACEHOLDER_BOT2_ID", &request.bot2_id);
    let template = template.replace("PLACEHOLDER_BOT2_NAME", &request.bot2_name);
    let template = template.replace("PLACEHOLDER_BOT2_CONTROLLER", &bot2_controller);
    let template = template.replace("PLACEHOLDER_BOT2_COMMAND", &bot2_command);
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

fn select_bot_directories(run_type: &str, config: &ControllerConfig, request: &MatchRequest) -> (String, String) {
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

    (bot1_directory, bot2_directory)
}

fn select_bot_controllers(config: &ControllerConfig, request: &MatchRequest) -> (String, String) {
    let bot1_controller = if request.bot1_base.is_empty() {
        config.bot_controller.clone()
    } else {
        request.bot1_base.clone()
    };

    let bot2_controller = if request.bot2_base.is_empty() {
        config.bot_controller.clone()
    } else {
        request.bot2_base.clone()
    };

    (bot1_controller, bot2_controller)
}

fn select_bot_commands(config: &ControllerConfig, request: &MatchRequest) -> (String, String) {
    let bot1_path = format!("{}/{}", config.bots_directory, request.bot1_name);
    let bot1_command = if !request.bot1_base.is_empty() && Path::new(&bot1_path).exists() {
        construct_bot_command(&request.bot1_type, &request.bot1_name, "10001", &request.bot2_id)
    } else {
        "null".to_string()
    };

    let bot2_path = format!("{}/{}", config.bots_directory, request.bot2_name);
    let bot2_command = if !request.bot2_base.is_empty() && Path::new(&bot2_path).exists() {
        construct_bot_command(&request.bot2_type, &request.bot2_name, "10002", &request.bot1_id)
    } else {
        "null".to_string()
    };

    (bot1_command, bot2_command)
}

fn construct_bot_command(bot_type: &String, bot_name: &String, game_port: &str, opponent_id: &String) -> String {
    let command = match bot_type.as_str() {
        "cppwin32" => format!("wine {bot_name}.exe"),
        "cpplinux" => format!("./{bot_name}"),
        "dotnetcore" => format!("dotnet {bot_name}.dll"),
        "java" => format!("java -jar {bot_name}.jar"),
        "linux" => format!("./{bot_name}"),
        "nodejs" => format!("node {bot_name}.js"),
        "python" => "python run.py".to_string(),
        _ => format!("./{bot_name}"),
    };

    format!(
        "sh -c \"cd /bot/ && {command} \
         --GamePort {game_port} --LadderServer 172.18.0.4 \
         --StartPort {game_port} --OpponentId {opponent_id} \
         > /bot/logs/stdout.log 2> /bot/logs/stderr.log\""
    )
}
