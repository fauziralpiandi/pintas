use std::fs;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{exit, Command as OsCommand};

const CONFIG_FILENAME: &str = "pintas.toml";

#[derive(Parser)]
#[command(name = "pintas")]
#[command(about = "A command alias manager for the terminal", long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  Run {
    #[arg(required = true)]
    alias: String,
  },
  List,
  Add {
    #[arg(required = true)]
    alias: String,
    #[arg(required = true)]
    command: String,
  },
  Remove {
    #[arg(required = true)]
    alias: String,
  },
  Edit {
    #[arg(required = true)]
    alias: String,
    #[arg(required = true)]
    command: String,
  },
}

#[derive(Deserialize, Serialize, Default)]
struct Config {
  aliases: HashMap<String, String>,
}

fn main() {
  let cli = Cli::parse();
  let result = match &cli.command {
    Commands::Run { alias } => run_alias(alias),
    Commands::List => list_aliases(),
    Commands::Add { alias, command } => add_alias(alias, command),
    Commands::Remove { alias } => remove_alias(alias),
    Commands::Edit { alias, command } => edit_alias(alias, command),
  };

  if let Err(e) = result {
    eprintln!("Error: {}", e);
    
    exit(1);
  }
}

fn load_config() -> Result<Config, String> {
  let content = fs::read_to_string(CONFIG_FILENAME)
    .map_err(|_| format!("Configuration file '{}' not found.", CONFIG_FILENAME))?;

  toml::from_str(&content).map_err(|e| format!("Failed to parse '{}'. {}", CONFIG_FILENAME, e))
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

fn run_alias(alias: &str) -> Result<(), String> {
  let config = load_config()?;
  let command_to_run = config
    .aliases
    .get(alias)
    .ok_or_else(|| format!("Alias '{}' not found.", alias))?;

  println!("Executing command: '{}'", command_to_run);

  let status = OsCommand::new("sh")
    .arg("-c")
    .arg(command_to_run)
    .status()
    .map_err(|e| format!("Failed to execute command: {}", e))?;

  if !status.success() {
    return Err(format!(
      "Command finished with an error (exit code: {})",
      status
    ));
  }

  Ok(())
}

fn add_alias(alias: &str, command: &str) -> Result<(), String> {
  let mut config = load_config().unwrap_or_default();

  config
    .aliases
    .insert(alias.to_string(), command.to_string());

  let toml_string =
    toml::to_string(&config).map_err(|e| format!("Failed to serialize configuration. {}", e))?;

  fs::write(CONFIG_FILENAME, toml_string)
    .map_err(|e| format!("Failed to write to '{}'. {}", CONFIG_FILENAME, e))?;

  println!("Successfully added alias '{}'.", alias);

  Ok(())
}

fn remove_alias(alias: &str) -> Result<(), String> {
  let mut config = load_config()?;

  if config.aliases.remove(alias).is_some() {
    let toml_string =
      toml::to_string(&config).map_err(|e| format!("Failed to serialize configuration. {}", e))?;

    fs::write(CONFIG_FILENAME, toml_string)
      .map_err(|e| format!("Failed to write to '{}'. {}", CONFIG_FILENAME, e))?;

    println!("Successfully removed alias '{}'.", alias);

    Ok(())
  } else {
    Err(format!("Alias '{}' not found.", alias))
  }
}

fn edit_alias(alias: &str, new_command: &str) -> Result<(), String> {
  let mut config = load_config()?;

  if config.aliases.contains_key(alias) {
    config
      .aliases
      .insert(alias.to_string(), new_command.to_string());

    let toml_string =
      toml::to_string(&config).map_err(|e| format!("Failed to serialize configuration. {}", e))?;

    fs::write(CONFIG_FILENAME, toml_string)
      .map_err(|e| format!("Failed to write to '{}'. {}", CONFIG_FILENAME, e))?;

    println!("Successfully edited alias '{}'.", alias);

    Ok(())
  } else {
    Err(format!("Alias '{}' not found. Cannot edit.", alias))
  }
}
