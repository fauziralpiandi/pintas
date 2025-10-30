use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::{Command as OsCommand, exit};

use std::path::PathBuf;

const CONFIG_FILENAME: &str = "pintas.toml";

fn get_pintas_dir() -> Result<PathBuf> {
    let home = env::var("HOME").context("Failed to get HOME directory from environment")?;

    Ok(PathBuf::from(home).join(".pintas"))
}

fn get_shims_dir() -> Result<PathBuf> {
    Ok(get_pintas_dir()?.join("shims"))
}

fn sync_shims(config: &Config) -> Result<()> {
    let pintas_path = env::current_exe().context("Failed to get current executable path")?;
    let shims_dir = get_shims_dir()?;

    fs::create_dir_all(&shims_dir).context("Failed to create shims directory")?;

    // a simple approach to remove all shims before recreating them
    // less efficient than comparing, but simpler and more robust
    for entry in fs::read_dir(&shims_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            fs::remove_file(path)?;
        }
    }

    for alias in config.aliases.keys() {
        let shim_path = shims_dir.join(alias);
        let shim_content = format!(
            "#!/bin/sh\nexec \"{}\" run --internal \"{}\" \"$@\"",
            pintas_path.to_string_lossy(),
            alias
        );

        fs::write(&shim_path, shim_content)?;

        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(&shim_path, fs::Permissions::from_mode(0o755))?;
    }

    // println!("Successfully synced aliases.");

    Ok(())
}

#[derive(Parser)]
#[command(name = "pintas")]
#[command(about = "A lightning-fast command alias manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
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
    Sync,
}

#[derive(Deserialize, Serialize, Default, Clone)]
struct Config {
    aliases: HashMap<String, String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Err(e) = run_command(cli.command) {
        eprintln!("Error: {:?}", e);

        exit(1);
    }

    Ok(())
}

fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Run {
            alias,
            args,
            internal,
        } => run_alias(alias, args, internal),
        Commands::Init { shell } => init_shell(&shell),
        Commands::List => run_readonly_command(command),
        Commands::Sync => sync_shims(&load_config()?),
        Commands::Add { .. } | Commands::Edit { .. } | Commands::Remove { .. } => {
            run_mutating_command(command)
        }
    }
}

fn run_readonly_command(command: Commands) -> Result<()> {
    let config = load_config()?;

    match command {
        Commands::List => list_aliases(&config),
        _ => unreachable!(),
    }
}

fn run_mutating_command(command: Commands) -> Result<()> {
    let mut config = if let Commands::Add { .. } = command {
        load_config().unwrap_or_default()
    } else {
        load_config()?
    };

    match command {
        Commands::Add { alias, command } => add_alias(&mut config, &alias, &command)?,
        Commands::Edit { alias, command } => edit_alias(&mut config, &alias, &command)?,
        Commands::Remove { alias } => remove_alias(&mut config, &alias)?,
        _ => unreachable!(),
    }

    save_config(&config)?;
    sync_shims(&config)
}

fn init_shell(shell: &str) -> Result<()> {
    let shims_dir = get_shims_dir()?;

    fs::create_dir_all(&shims_dir).context("Failed to create shims directory")?;

    match shell {
        "bash" => {
            println!(
                "# pintas shell integration for bash\n#\n# Add the following line to your ~/.bashrc or ~/.profile:\n#\n  export PATH=\"{}\":$PATH\n",
                shims_dir.to_string_lossy()
            );

            Ok(())
        }

        _ => Err(anyhow!("Shell '{}' not supported.", shell)),
    }
}

fn load_config() -> Result<Config> {
    let content = fs::read_to_string(CONFIG_FILENAME)
        .with_context(|| format!("Configuration file '{}' not found.", CONFIG_FILENAME))?;

    toml::from_str(&content).with_context(|| format!("Failed to parse '{}'.", CONFIG_FILENAME))
}

fn save_config(config: &Config) -> Result<()> {
    let toml_string = toml::to_string(config).context("Failed to serialize configuration.")?;

    fs::write(CONFIG_FILENAME, toml_string)
        .with_context(|| format!("Failed to write to '{}'.", CONFIG_FILENAME))?;

    Ok(())
}

fn list_aliases(config: &Config) -> Result<()> {
    println!("Available aliases:");

    if config.aliases.is_empty() {
        println!("No aliases found.");
    } else {
        let mut sorted_aliases: Vec<_> = config.aliases.iter().collect();

        sorted_aliases.sort_by(|a, b| a.0.cmp(b.0));

        for (alias, command) in sorted_aliases {
            println!(" - {}: \"{}\"", alias, command);
        }
    }

    Ok(())
}

fn run_alias(alias: String, args: Vec<String>, internal: bool) -> Result<()> {
    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(_) if internal => exit(126), // config not found, so alias can't exist
        Err(e) => return Err(e).context("Failed to load pintas config"),
    };

    let command_to_run = match config.aliases.get(&alias) {
        Some(cmd) => cmd,
        None if internal => exit(126), // alias not found
        None => return Err(anyhow!("Alias '{}' not found.", alias)),
    };

    if !internal {
        println!("Executing command: '{}'", command_to_run);
    }

    let mut cmd = OsCommand::new("sh");

    cmd.arg("-c");
    cmd.arg(command_to_run);
    cmd.arg(alias); // this becomes $0 in the script
    cmd.args(args); // these become $1, $2, ...

    let status = cmd.status().context("Failed to execute command")?;

    if internal {
        exit(status.code().unwrap_or(1));
    }

    if !status.success() {
        return Err(anyhow!(
            "Command finished with an error (exit code: {})\n",
            status
        ));
    }

    Ok(())
}

fn add_alias(config: &mut Config, alias: &str, command: &str) -> Result<()> {
    if config.aliases.contains_key(alias) {
        return Err(anyhow!(
            "Alias '{}' already exists. Use 'edit' to modify it.",
            alias
        ));
    }

    config
        .aliases
        .insert(alias.to_string(), command.to_string());

    println!("Successfully added alias '{}'.", alias);

    Ok(())
}

fn edit_alias(config: &mut Config, alias: &str, new_command: &str) -> Result<()> {
    if config.aliases.contains_key(alias) {
        config
            .aliases
            .insert(alias.to_string(), new_command.to_string());

        println!("Successfully edited alias '{}'.", alias);

        Ok(())
    } else {
        Err(anyhow!("Alias '{}' not found. Cannot edit.", alias))
    }
}

fn remove_alias(config: &mut Config, alias: &str) -> Result<()> {
    if config.aliases.remove(alias).is_some() {
        println!("Successfully removed alias '{}'.", alias);

        Ok(())
    } else {
        Err(anyhow!("Alias '{}' not found.", alias))
    }
}
