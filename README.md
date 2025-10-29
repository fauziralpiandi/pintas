# Pintas

> Your lightning-fast command alias manager.

`pintas` is a simple, fast, and powerful command-line tool written in Rust that helps you manage and use shortcuts for your long and repetitive shell commands.

The name *"pintas"* is the Indonesian word for *"shortcut"*. It is also a recursive acronym for *"perintah ringkas"*, which translates to *"Concise Commands"*.

## Configuration

Your aliases are stored in a `pintas.toml` file in the same directory where you run the tool:

```toml
[aliases]
build-release = "cargo build --release"
start-server = "docker-compose up -d"
update-system = "sudo apt update && sudo apt upgrade -y"
```

## Usage

Available commands:

### List aliases

Lists all saved aliases.

```sh
pintas list
```

### Run an alias

Executes the command associated with an alias.

```sh
pintas run <alias>
```

### Add a new alias

Adds a new alias to your `pintas.toml` file. If the alias already exists, it will be overwritten.

```sh
pintas add my-new-alias "echo 'Hello from Pintas!'"
```

### Edit an alias

Changes the command for an existing alias.

```sh
pintas edit my-new-alias "echo 'A new command for an old alias.'"
```

### Remove an alias

Deletes an alias from your `pintas.toml` file.

```sh
pintas remove my-new-alias
```
