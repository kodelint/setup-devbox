# setup-devbox

![Rust Logo](https://img.shields.io/badge/Rust-red?style=for-the-badge&logo=rust)
![YAML Config](https://img.shields.io/badge/YAML-blue?style=for-the-badge&logo=yaml)
![Platform](https://img.shields.io/badge/Platform-macOS-blue?style=for-the-badge&logo=apple)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/kodelint/setup-devbox/actions/workflows/release.yml/badge.svg)](https://github.com/kodelint/setup-devbox/actions/workflows/release.yml)
[![GitHub release](https://img.shields.io/github/release/kodelint/setup-devbox.svg)](https://github.com/kodelint/setup-devbox/releases)
[![GitHub stars](https://img.shields.io/github/stars/kodelint/setup-devbox.svg)](https://github.com/kodelint/setup-devbox/stargazers)
[![Last commit](https://img.shields.io/github/last-commit/kodelint/setup-devbox.svg)](https://github.com/kodelint/setup-devbox/commits/main)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/kodelint/setup-devbox/pulls)

<img src="https://github.com/kodelint/kodelint.github.io/blob/594a8f7311a2355c83327e9765c6a5ee4d2afa23/assets/uploads/01-setup-devbox.png" alt="setup-devbox Image" width="500" height="500" style="display: block; margin: 0 auto;">

## 🚀 Accelerate Your Development Environment Setup

`setup-devbox` is a powerful, opinionated, and highly configurable command-line tool designed to streamline the provisioning of your development environment. Say goodbye to manual installations and inconsistent setups across your machines!

By defining your desired tools, system settings, shell configurations, and fonts in simple YAML files, `setup-devbox` automates the entire process, ensuring a reproducible and consistent development workstation every time.

---

## ✨ Features

`setup-devbox` acts as your personal environment orchestrator, offering:

- **Declarative Configuration**: Define your entire development environment in easy-to-read YAML files.
- **Intelligent State Management**: Tracks installed tools and applied settings in a `state.json` file to prevent redundant operations and ensure efficiency.
- **Platform Support**: Currently designed for and tested on **macOS**. Linux support is planned for future releases.
- **Smart Update Policies**: Control when tools with version "latest" should be updated using the `update_latest_only_after` configuration.
  Override update policies with the `--update-latest` flag to force updates of all "latest" version tools.
- **Smart Tool Configuration Management**: Manage tool configuration files and tracks the drifts using `SHA256`
- **Extensible Installer Support**:
  - 📦 **Homebrew (`brew`)**: Install packages and applications (primarily macOS).
  - 🐙 **GitHub Releases (`github`)**: Download and install pre-compiled binaries.
  - ⚙️ **Go (`go`)**: Install Go binaries and tools.
  - 🦀 **Cargo (`cargo`)**: Install Rust crates.
  - 🐍 **Pip (`pip`)**: Install Python packages.
  - 🦀 **Rustup (`rustup`)**: Manage and install Rust toolchains and components.
  - 🚀 **URL (`direct URL`)**: Manage and install tool directly from URL.
  -  **uv (`uv`)**: UV Installer to manage `python` version for the system.
- **Highly Modular and Pluggable**: The architecture is designed for ease of extension. Adding support for new package managers or installation methods is straightforward, requiring minimal changes to the core logic and making `setup-devbox` adaptable to evolving needs.
- **System Settings Application**: Define macOS system preferences to be applied automatically.
- **Shell Configuration Management**: Manage shell aliases, environment variables, and dotfiles.
- **Font Installation**: Install and manage custom fonts for your terminal and editor.
- **Idempotent Operations**: Run the tool multiple times without side effects; it only applies changes if necessary.
- **Detailed Logging**: Provides clear feedback on installation progress and potential issues.

---

## 🛠️ Installation

To get `setup-devbox` up and running, you'll need the Rust toolchain installed. If you don't have Rust, you can install it via `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
```

Once Rust is installed, you can clone this repository and build `setup-devbox`:

```bash
git clone https://github.com/kodelint/setup-devbox.git
cd setup-devbox
cargo build --release
```

The compiled executable will be located at `./target/release/setup-devbox`. You might want to move it to a directory in your system's `PATH`, e.g.:

```bash
sudo mv ./target/release/setup-devbox /usr/local/bin/
```

## 🚀 Usage

`setup-devbox` primarily operates through a main `config.yaml` file, which points to other configuration files that define your desired environment.

| Command         | Description                                                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `now`           | Installs and configures tools, fonts, OS settings, and shell.                                                                              |
| `generate`      | Generates default configuration files.                                                                                                     |
| `sync-config`   | Synchronizes or generates configurations from a state file.                                                                                |
| `edit`          | Edits configuration files or the state file in your editor.                                                                                |
| `add`           | Adds a new tool, font, setting, or alias.                                                                                                  |
| `remove`        | Removes an installed tool, font, alias, or setting.                                                                                        |
| `reset`         | Resets the installation state.                                                                                                             |
| `check-updates` | Checks for updates for all tools defined in `tools.yaml` and displays them in two tables: "Updates Available" and "Manual Check Required". |
| `help`          | Shows detailed help for commands and installers.                                                                                           |
| `version`       | Shows the current version of the tool.                                                                                                     |

##### _(Note: Details commands are documented in [Commands](COMMANDS.md))_

## ⚙️ Configuration Examples

### `tools.yaml`

```yaml
update_latest_only_after: 7d

tools:
  ## Github Source
  - name: zed
    version: 0.225.13
    source: github
    repo: zed-industries/zed
    tag: v0.225.13
    rename_to: zed
    configuration_manager:
      enabled: true
      tools_configuration_paths:
        - $HOME/.config/zed/settings.json
        - $HOME/.config/zed/keymap.json

  # Manages Rust toolchains and components.
  - name: rust
    source: rustup
    version: stable
    options:
      - rust-src
      - rust-docs
      - rustfmt
      - rust-analyzer
      - clippy

  ## Cargo Source
  - name: atuin
    version: 18.12.1
    source: cargo
    configuration_manager:
      enabled: true
      tools_configuration_paths:
        - $HOME/.config/atuin/config.toml

  - name: lsd
    version: 1.2.0
    source: cargo
    configuration_manager:
      enabled: true
      tools_configuration_paths:
        - $HOME/.config/lsd/config.toml
        - $HOME/.config/lsd/colors.toml
        - $HOME/.config/lsd/icons.toml

  - name: bat
    version: 0.26.1
    source: cargo
  - name: cargo-pants
    version: 0.4.38
    source: cargo
  - name: cargo-edit
    version: 0.13.9
    source: cargo
  - name: cargo-watch
    version: 8.5.3
    source: cargo
  - name: cargo-outdated
    version: 0.17.0
    source: cargo
  - name: cargo-machete
    version: 0.9.1
    source: cargo
  - name: cargo-audit
    version: 0.22.1
    source: cargo

  - name: git-cliff
    version: 2.12.0
    source: cargo
  - name: git-delta
    version: 0.18.2
    source: cargo
  - name: just
    version: 1.46.0
    source: cargo
  - name: uv
    version: 0.10.8
    source: cargo

  ## Homebrew Source
  - name: rustup
    source: brew

  - name: ghostty
    source: brew
    configuration_manager:
      enabled: true
      tools_configuration_paths:
        - $HOME/.config/ghostty/config

  - name: helix
    source: brew
    configuration_manager:
      enabled: true
      tools_configuration_paths:
        - $HOME/.config/helix/config.toml
        - $HOME/.config/helix/languages.toml
```

### Update Policy Behavior

The `update_latest_only_after` feature provides intelligent control over when tools with version "**latest**" should be updated:

- **Specific Versions:** Tools with explicit versions (e.g., v2.50.0, 24.4.2) are always updated when the version changes
- **Latest Versions:** Tools with version: latest are only updated if:
  - They've never been installed before, OR
  - Their last update was more than the specified duration ago
- **No Version Specified:** Tools without a version field are treated as "latest" and follow the update policy

#### Supported Duration Formats:

- 1d or 7d
- 24h
- 60m

_(**Note:** Override update policies with the `--update-latest` flag to force updates of all "latest" version tools.)_

### 💾 Backup Configuration

`setup-devbox` automatically creates zip backups of your configuration files before major changes. You can control the backup behavior using the following environment variables:

- `SDB_CONFIG_BACKUP_RETENTION`: Sets the number of old backups to keep (default: 7). Older backups are automatically discarded.
- `SDB_CONFIG_BACKUP_RETENTION_PATH`: Specifies a custom directory path where the backup zip files will be stored. If not set, it defaults to a `.backup` folder inside your configuration directory.

### `fonts.yaml`

```yaml
fonts:
  - name: 0xProto
    version: 3.4.0
    source: github
    repo: ryanoasis/nerd-fonts
    tag: v3.4.0
    install_only: ["regular", "Mono"]
  - name: JetBrainsMono
    version: 3.4.0
    source: github
    repo: ryanoasis/nerd-fonts
    tag: v3.4.0
    install_only: ["Regular"]
```

### `shellrc.yaml`

```yaml
# shellrc.yaml - Shell configuration for setup-devbox
# This file defines shell run commands organized by sections
run_commands:
  shell: "zsh" # or "bash"
  run_commands:
    # Exports Section - Environment variables
    - command: |
        export EDITOR="zed"
        export VISUAL="zed"
      section: Exports

    # Paths Section - PATH modifications
    - command: export SDB_CONFIG_PATH="$HOME/Documents/SDB"
      section: Paths
    - command: export SDB_CONFIG_BACKUP_RETENTION=5
      section: Paths
    - command: export PATH="$HOME/bin:$PATH"
      section: Paths
    - command: export PATH="$HOME/.rustup/toolchains/stable-x86_64-apple-darwin/bin:$PATH"
      section: Paths
    - command: export PATH="$HOME/.cargo/bin:$PATH"
      section: Paths
    - command: export BAT_CONFIG_PATH="$HOME/.config/bat/config.toml"
      section: Paths

    # Evals Section - Command evaluations
    - command: eval "$(starship init zsh)"
      section: Evals
    - command: eval "$(atuin init zsh --disable-up-arrow)"
      section: Evals
    # Other Section - Miscellaneous configurations
    - command: source $HOME/.config/secrets.zsh
      section: Other

aliases:
  - name: cat # Replace `cat with `bat`
    value: bat --paging=never --theme "TwoDark" --style "numbers,changes,header"
  - name: config
    value: zed $HOME/.config
  - name: ls
    value: lsd
  - name: sdb
    value: setup-devbox
```

### `settings.yaml`

```yaml
settings:
  macos:
    - domain: NSGlobalDomain
      key: AppleShowAllExtensions
      value: "true"
      type: bool
    - domain: com.apple.finder
      key: AppleShowAllFiles
      value: "true"
      type: bool
```

## 🔧 Configuration Manager:

`setup-devbox` features a sophisticated Configuration Manager that ensures your tool configurations remain consistent and
version-controlled. This powerful subsystem detects and corrects configuration drift, maintaining your development
environment's integrity across installations and updates.

```yaml
tools:
  - name: lsd
    source: cargo
    version: 1.2.0
    configuration_manager: # 🎛️ Tool's configuration manager
      enabled: true # ✅ Enable configuration management
      tools_configuration_paths: # 📁 Tools configuration paths to manage
        - $HOME/.config/lsd/config.yaml # 🔧 Main configuration file
        - $HOME/.config/lsd/icons.yaml # 🎨 Icons configuration file
```

### 🚀 How It Works

#### 📊 Configuration Management Flow

```text
Source Template → Validation → Transformation → Deployment → State Tracking
     ↓               ↓             ↓             ↓              ↓
   TOML/JSON      Syntax Check   Format Convert  Copy to Dest   SHA256 Tracking
     ↓               ↓             ↓             ↓              ↓
  ~/.setup-devbox/  ✅ Valid     → YAML/JSON   → ~/.config/   → State Database
  configs/tools/               (Target Format)    tool/
```

#### 🔍 Key Features

- **🔒 SHA256 Hashing:** Tracks both source and destination file checksums
- **📊 Drift Reporting:** Provides detailed reports on configuration differences
- **🔄 Auto-Correction:** Automatically synchronizes configurations when drift is detected

#### 📁 Smart Source File Resolution

The system intelligently locates configuration source files using this priority order:

1. Environment Variables (Highest Priority):

   ```bash
    export SBD_CONFIG_PATH="/custom/source_config_path"
    export SBD_TOOL_CONFIGURATION_PATH="/custom/source_config_path/configs/tools/"
   ```

   > > 📝 **Important Notes:**
   >
   > - `SBD_CONFIG_PATH`: Root configuration directory for entire setup-devbox
   > - `SBD_TOOL_CONFIGURATION_PATH`: Specific directory for tool configuration files
   > - Both expect organized folder structure with tool-specific subdirectories

   > > 📋 Example Resolution:
   >
   > - Destination file: $HOME/.config/lsd/config.yaml
   > - SBD_CONFIG_PATH: $HOME/Documents/SDB
   > - Expected source file locations:
   >   - $SBD_CONFIG_PATH/configs/tools/lsd/config.toml
   >   - $SBD_CONFIG_PATH/configs/tools/lsd/icons.toml
   > - File naming convention:
   >   - Source: config.toml (TOML format) → Destination: config.yaml (YAML format)

2. Default Locations (Fallback):
   ```bash
   $HOME/.setup-devbox/configs/tools/<<tool_name>>/
   ```

#### Advanced Multi-File Management

```yaml
tools:
  - name: nvim
    source: brew
    configuration_manager:
      enabled: true
      tools_configuration_paths:
        - $HOME/.config/nvim/init.vim
        - $HOME/.config/nvim/coc-settings.json
        - $HOME/.config/nvim/lua/plugins.lua
      additional_cmd:
        - nvim --headless +'checkhealth' +qall # 🛠️ Pre-apply validation
```

### 📊 State Management

This is how the `state.json` files looks

```json
  "lsd": {
    "version": "1.2.0",
    "install_path": "/Users/kodelint/.cargo/bin/lsd",
    "installed_by_devbox": true,
    "install_method": "cargo-install",
    "renamed_to": null,
    "package_type": "rust-crate",
    "repo": null,
    "tag": null,
    "options": null,
    "url": null,
    "last_updated": "2025-09-05T16:45:26.737584+00:00",
    "executable_path_after_extract": null,
    "additional_cmd_executed": null,
    "configuration_manager": {
      "enabled": true,
      "tools_configuration_paths": [
        "$HOME/.config/lsd/config.yaml",
        "$HOME/.config/lsd/icons.yaml"
      ],
      "source_configuration_sha": "66ce9065be901....",
      "destination_configuration_sha": "d56d715d7072..."
    }
  },
```

### 🔔 Drift Detection Alerts

```bash
  ...
  ...
  TOOLS:
  =======
  [INFO] [Tools] Configuration Management...
  [INFO] [Tools] Updating configuration for: Ghostty
  [INFO] [Tools] Configuration written to: /Users/kodelint/.config/ghostty/config
```

### ✅ Recommended Configuration Structure

```bash
>> ls --tree
 .
├──  configs
│   ├──  config.yaml
│   ├──  fonts.yaml
│   ├──  settings.yaml
│   ├──  shellrc.yaml
│   ├──  tools
│   │   ├──  atuin
│   │   │   ├──  config.toml
│   │   │   └──  db
│   │   │       ├──  history.db
│   │   │       ├──  history.db-wal
│   │   │       ├──  key
│   │   │       ├──  meta.db
│   │   │       ├──  records.db
│   │   │       ├──  records.db-shm
│   │   │       └──  records.db-wal
│   │   ├──  ghostty
│   │   │   └──  config.toml
│   │   ├──  helix
│   │   │   ├──  config.toml
│   │   │   └──  languages.toml
│   │   ├──  lsd
│   │   │   ├──  colors.toml
│   │   │   ├──  config.toml
│   │   │   └──  icons.toml
│   │   └──  zed
│   │       ├──  keymap.toml
│   │       └──  settings.toml
│   └──  tools.yaml
└──  state.json
```

The Configuration Manager ensures your development tools maintain consistent settings across all your machines,
providing enterprise-grade configuration management with developer-friendly simplicity. 🚀

## 🤝 Contributing

Contributions are welcome! If you find a bug, have a feature request, or want to contribute code, please open an issue or submit a pull request.
