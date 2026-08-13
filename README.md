# blob

<img src="icon.png" alt="BlobVault" width="96" height="96">

Local password manager (BlobVault).

## Features

- Encrypted vault on disk (Argon2 key derivation + XChaCha20-Poly1305)
- Master password to create and unlock the vault
- Add, edit, and delete account entries (name, username, password)
- Search accounts by name or username
- Reveal/hide passwords and copy username or password to the clipboard
- Built-in random password generator
- Lock the vault without quitting
- Reset vault if the master password is forgotten (permanently deletes all data)
- Sensitive values cleared from memory when no longer needed
- Atomic vault saves (temp file + rename) to avoid corruption on crash

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- On Windows: the MSVC build tools (`rustup default stable-x86_64-pc-windows-msvc`)

## Build

Debug:

```bash
cargo build
```

Release (stripped, LTO enabled via `Cargo.toml`):

```bash
cargo build --release
```

The binary is written to:

- Windows: `target/release/blobvault.exe`
- Linux/macOS: `target/release/blobvault`

Run it locally:

```bash
cargo run --release
```

## Ship a GitHub Release

1. Bump `version` in `Cargo.toml` if needed, then commit and push to `main`.

2. Create and push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

3. Build the release binary on the machine (or OS) you want to ship for:

```bash
cargo build --release
```

4. On GitHub, open the repo → **Releases** → **Draft a new release**:
   - Choose the tag you just pushed (e.g. `v0.1.0`)
   - Set a title and release notes
   - Attach the binary from `target/release/` (for Windows, `blobvault.exe`)
   - Publish the release

To ship for more than one OS, build on each platform (or use a cross-compile setup) and attach each binary to the same release.
