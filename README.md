# Pintas

> A lightning-fast command alias manager written in Rust

"Pintas" (also a recursive acronym for "perintah ringkas") means "shortcut" or "concise commands" in Indonesian.

## Usage

- `pintas list`: Show all aliases.
- `pintas init <shell>`: Generate the shell integration script.
- `pintas run <alias> [args...]`: Execute an alias.
- `pintas add <alias> <command>`: Add a new alias.
- `pintas edit <alias> <command>`: Change an existing alias.
- `pintas remove <alias>`: Delete an alias.

Aliases are stored in `pintas.toml`.

## Shell Integration (Optional)

To run aliases directly (e.g. `myalias` instead of `pintas run myalias`), add this to `.bashrc`:

```bash
eval "$(./target/debug/pintas init bash)"
```

This also allows passing arguments to aliases. Arguments are available in alias command as `$1`, `$2`, etc.

**Example:**

- Add alias: `pintas add greet "echo Hello, $1"`
- Run in shell: `greet World`
- Output: `Hello, world!`
