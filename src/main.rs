use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::{Command as OsCommand, exit};

const CONFIG_FILENAME: &str = "pintas.toml";

#[derive(Parser)]
#[command(name = "pintas")]
#[command(about = "A lightning-fast command alias manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(required = true)]
        alias: String,
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
        #[arg(long, hide = true)]
        internal: bool,
    },
    Init {
        #[arg(required = true)]
        shell: String,
    },
    List,
    Add {
        #[arg(required = true)]
        alias: String,
        #[arg(required = true)]
        command: String,
    },
    Edit {
        #[arg(required = true)]
        alias: String,
        #[arg(required = true)]
        command: String,
    },
    Remove {
        #[arg(required = true)]
        alias: String,
    },
}

#[derive(Deserialize, Serialize, Default)]
struct Config {
    aliases: HashMap<String, String>,
}

fn main() {
    let cli = Cli::parse();
    let result = match &cli.command {
        Commands::Run {
            alias,
            args,
            internal,
        } => run_alias(alias, args, *internal),
        Commands::Init { shell } => init_shell(shell),
        Commands::List => list_aliases(),
        Commands::Add { alias, command } => add_alias(alias, command),
        Commands::Edit { alias, command } => edit_alias(alias, command),
        Commands::Remove { alias } => remove_alias(alias),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

fn init_shell(shell: &str) -> Result<(), String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?
        .to_string_lossy()
        .to_string();

    match shell {
        "bash" => {
            println!(
                r#"
# pintas shell integration for bash
# Add the following line to your ~/.bashrc:
#   eval "$("{pintas_path}" init bash)"

command_not_found_handler() {{
  "{pintas_path}" run --internal "$@"
  local exit_code=$?

  if [ $exit_code -eq 126 ]; then
    printf 'bash: %s: command not found\n' "$1" >&2
    return 127
  else
    return $exit_code
  fi
}}
"#,
                pintas_path = current_exe
            );
            Ok(())
        }
        _ => Err(format!("Shell '{}' not supported.", shell)),
    }
}

fn load_config() -> Result<Config, String> {
    let content = fs::read_to_string(CONFIG_FILENAME)
        .map_err(|_| format!("Configuration file '{}' not found.", CONFIG_FILENAME))?;

    toml::from_str(&content).map_err(|e| format!("Failed to parse '{}'. {}", CONFIG_FILENAME, e))
}

fn save_config(config: &Config) -> Result<(), String> {
    let toml_string =
        toml::to_string(config).map_err(|e| format!("Failed to serialize configuration. {}", e))?;

    fs::write(CONFIG_FILENAME, toml_string)
        .map_err(|e| format!("Failed to write to '{}'. {}", CONFIG_FILENAME, e))?;

    Ok(())
}

fn list_aliases() -> Result<(), String> {
    let config = load_config()?;

    println!("Available aliases:");

    if config.aliases.is_empty() {
        println!("No aliases found.");
    } else {
        let mut sorted_aliases: Vec<_> = config.aliases.into_iter().collect();

        sorted_aliases.sort_by(|a, b| a.0.cmp(&b.0));

        for (alias, command) in sorted_aliases {
            println!(" - {}: \"{}\"", alias, command);
        }
    }

    Ok(())
}

fn run_alias(alias: &str, args: &[String], internal: bool) -> Result<(), String> {
    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(_) if internal => exit(126), // config not found, so alias can't exist
        Err(e) => return Err(e),
    };

    let command_to_run = match config.aliases.get(alias) {
        Some(cmd) => cmd,
        None if internal => exit(126), // alias not found
        None => return Err(format!("Alias '{}' not found.", alias)),
    };

    if !internal {
        println!("Executing command: '{}'", command_to_run);
    }

    let mut cmd = OsCommand::new("sh");

    cmd.arg("-c");
    cmd.arg(command_to_run);
    cmd.arg(alias); // this becomes $0 in the script
    cmd.args(args); // these become $1, $2, ...

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if internal {
        exit(status.code().unwrap_or(1));
    }

    if !status.success() {
        return Err(format!(
            "Command finished with an error (exit code: {})",
            status
        ));
    }

    Ok(())
}

fn add_alias(alias: &str, command: &str) -> Result<(), String> {
    let mut config = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            if e.contains("not found") {
                Config::default()
            } else {
                return Err(e);
            }
        }
    };

    if config.aliases.contains_key(alias) {
        return Err(format!(
            "Alias '{}' already exists. Use 'edit' to modify it.",
            alias
        ));
    }

    config
        .aliases
        .insert(alias.to_string(), command.to_string());

    save_config(&config)?;

    println!("Successfully added alias '{}'.", alias);

    Ok(())
}

fn edit_alias(alias: &str, new_command: &str) -> Result<(), String> {
    let mut config = load_config()?;

    if config.aliases.contains_key(alias) {
        config
            .aliases
            .insert(alias.to_string(), new_command.to_string());

        save_config(&config)?;

        println!("Successfully edited alias '{}'.", alias);

        Ok(())
    } else {
        Err(format!("Alias '{}' not found. Cannot edit.", alias))
    }
}

fn remove_alias(alias: &str) -> Result<(), String> {
    let mut config = load_config()?;

    if config.aliases.remove(alias).is_some() {
        save_config(&config)?;

        println!("Successfully removed alias '{}'.", alias);

        Ok(())
    } else {
        Err(format!("Alias '{}' not found.", alias))
    }
}
