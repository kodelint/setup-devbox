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

## 🚀 Accelerate Your Development Environment Setup

`setup-devbox` is a powerful, opinionated, and highly configurable command-line tool designed to streamline the provisioning of your development environment. Say goodbye to manual installations and inconsistent setups across your machines!

By defining your desired tools, system settings, shell configurations, and fonts in simple YAML files, `setup-devbox` automates the entire process, ensuring a reproducible and consistent development workstation every time.

---

## ✨ Features

`setup-devbox` acts as your personal environment orchestrator, offering:

* **Declarative Configuration**: Define your entire development environment in easy-to-read YAML files.
* **Intelligent State Management**: Tracks installed tools and applied settings in a `state.json` file to prevent redundant operations and ensure efficiency.
* **Platform Support**: Currently designed for and tested on **macOS**. Linux support is planned for future releases.
* **Extensible Installer Support**:
    * 📦 **Homebrew (`brew`)**: Install packages and applications (primarily macOS).
    * 🐙 **GitHub Releases (`github`)**: Download and install pre-compiled binaries.
    * ⚙️ **Go (`go`)**: Install Go binaries and tools.
    * 🦀 **Cargo (`cargo`)**: Install Rust crates.
    * 🐍 **Pip (`pip`)**: Install Python packages.
    * 🧡 **Rustup (`rustup`)**: Manage and install Rust toolchains and components.
* **Highly Modular and Pluggable**: The architecture is designed for ease of extension. Adding support for new package managers or installation methods is straightforward, requiring minimal changes to the core logic and making `setup-devbox` adaptable to evolving needs.
* **System Settings Application** (Planned): Define macOS system preferences to be applied automatically.
* **Shell Configuration Management** (Planned): Manage shell aliases, environment variables, and dotfiles.
* **Font Installation** (Planned): Install and manage custom fonts for your terminal and editor.
* **Idempotent Operations**: Run the tool multiple times without side effects; it only applies changes if necessary.
* **Detailed Logging**: Provides clear feedback on installation progress and potential issues.

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

#### 1. Create your configuration directory:
By default, `setup-devbox` looks for its configurations in `~/.setup-devbox/configs/`.
```bash
mkdir -p ~/.setup-devbox/configs/
```
#### 2. Define your `config.yaml`:
This file tells `setup-devbox` where to find your other configuration files.
```yaml
# ~/.setup-devbox/configs/main_config.yaml
tools: "~/.setup-devbox/configs/tools.yaml"
settings: "~/.setup-devbox/configs/settings.yaml"
shellrc: "~/.setup-devbox/configs/shellrc.yaml"
fonts: "~/.setup-devbox/configs/fonts.yaml"
```
#### 3. Define your `tools.yaml` (and other config files):
This is where you specify the actual tools you want installed. See examples below.

#### 4. Run `setup-devbox`:
Execute the `now` command to provision your environment based on your configurations.
```bash
setup-devbox now
```

##### _(Note: Details commands are documented in [Commands](commands.md))_

## ⚙️ Configuration Examples 
### `tools.yaml`
```yaml
# ~/.setup-devbox/configs/tools.yaml

tools:
  # --- GitHub Release Installer Example (source: github) ---
  # Downloads pre-compiled binaries from GitHub releases.
  - name: git-pr                    # The name of the tool (e.g., 'terraform', 'kubectl')
    source: github                  # Specifies the GitHub installer.
    repo: kodelint/git-pr           # REQUIRED: The GitHub repository in 'owner/repo' format.
    version: 0.1.0                  # (Optional) The specific version to download.
    rename_to: git-pr               # (Optional) Rename the downloaded executable.
  - name: git-spellcheck            # The name of the tool (e.g., 'terraform', 'kubectl')
    source: github                  # Specifies the GitHub installer.
    repo: kodelint/git-spellcheck   # REQUIRED: The GitHub repository in 'owner/repo' format.
    version: 0.0.1                  # (Optional) The specific version to download.
    rename_to: git-spellcheck       # (Optional) Rename the downloaded executable.
  # --- Go Installer Example (source: go) ---
  # Installs Go binaries directly from their source via `go install`.
  - name: gh                    # The Go module path.
    source: go                  # Specifies the Go installer.
    version: v2.50.0            # (Optional) The version to install (e.g., "v2.50.0").
    rename_to: gh               # (Optional) Rename the resulting executable.
    options:
      - -ldflags="-s -w"        # Example: Pass build flags to `go install`.

  # --- Rustup Installer Example (source: rustup) ---
  # Manages Rust toolchains and components.
  - name: rust                  # A descriptive name for the Rust toolchain.
    source: rustup              # Specifies the Rustup installer.
    version: stable             # REQUIRED: The Rust toolchain (e.g., "stable", "nightly").
    options:
      - rust-src                # Example: Components to add (e.g., "rust-src", "clippy").
      - rust-docs

  # --- Cargo Installer Example (source: cargo) ---
  # Installs Rust crates from crates.io via `cargo install`.
  - name: cargo-watch           # The name of the Rust crate.
    source: cargo               # Specifies the Cargo installer.
    version: 8.5.1              # (Optional) The specific version of the crate.
    options:
      - --features="notify"     # Example: Enable specific features for the crate.

  # --- Pip Installer Example (source: pip) ---
  # Installs Python packages using `pip`.
  - name: black                 # The name of the Python package.
    source: pip                 # Specifies the Pip installer.
    version: 24.4.2             # (Optional) The specific version of the package.
    options:
      - --user                  # IMPORTANT: Installs package to user's home directory.
      - --upgrade               # Example: Always upgrade if already installed.

  # --- Homebrew Installer Example (source: brew) ---
  # Installs macOS/Linux packages using Homebrew.
  - name: git                   # The Homebrew formula name.
    source: brew                # Specifies the Homebrew installer.
    # version:                  # Not typically used for brew; versions are managed by formulae.
    options:
      - --without-completion    # Example: Pass flags to `brew install`.
```
### `fonts.yaml`
```yaml
fonts:
- name: 0xProto
  version: 3.4.0
  source: github
  repo: ryanoasis/nerd-fonts
  tag: v3.4.0
```

### `shellrc.yaml`
```yaml
shellrc:
  shell: zsh
  raw_configs:
    - export PATH=$HOME/bin:$PATH
    - export PATH=$HOME/.cargo/bin:$PATH
    - export PATH=$HOME/go/bin:$PATH
    - eval "$(starship init zsh)"
    - "[ -f ~/.fzf.zsh ] && source ~/.fzf.zsh"
    - zle -N fzf-history-widget
    - bindkey '^R' fzf-history-widget
    - 'export FZF_CTRL_R_OPTS="--no-preview --scheme=history --tiebreak=index --bind=ctrl-r:toggle-sort"'
    - |
      export FZF_CTRL_T_OPTS="
        --preview 'bat --style=numbers --color=always {} || cat {}'
        --preview-window=right:80%
        --bind ctrl-b:preview-page-up,ctrl-f:preview-page-down
      "
    - setopt append_history
    - setopt share_history
    - HISTSIZE=100000
    - SAVEHIST=100000
    - HISTFILE=~/.zsh_history
    - setopt hist_ignore_all_dups
aliases:
  - name: code
    value: cd $HOME/Documents/github/
  - name: gocode
    value: cd $HOME/go/src/
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

## 🤝 Contributing
Contributions are welcome! If you find a bug, have a feature request, or want to contribute code, please open an issue or submit a pull request.