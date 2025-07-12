## Commands
This document provides a comprehensive guide to the `setup-devbox` command-line interface (CLI) and its various commands.

Setup development environment with ease
setup-devbox is designed to automate the setup and configuration of your development environment. By leveraging declarative YAML files, it ensures a consistent and reproducible setup across different machines.

#### Usage:
```bash
setup-devbox [OPTIONS] <COMMAND>
```

### 🌐 Global Options
These options can be used with any setup-devbox command.

`-d`, `--debug`
#### **Description**:
This argument allows users to turn on debugging information. 
When enabled, `setup-devbox` will output more verbose logs, including detailed internal operations, variable states, and more granular progress messages. This is extremely helpful for troubleshooting issues or understanding exactly what the tool is doing.

**Usage:**
```bash
setup-devbox <COMMAND> --debug
setup-devbox now -d
```

`-h`, `--help`

#### **Description**:
Prints general help information for the `setup-devbox` tool, listing all available commands and global options. This is equivalent to setup-devbox help.

**Usage:**
```bash
setup-devbox --help
```

### 🚀 Available Commands
- #### `version` Show the current Version of the tool.

    ##### **Description**:
    This command simply outputs the version number of the setup-devbox application. It's useful for quickly checking which version you are running, especially when reporting issues or ensuring you have the latest updates.

    **Usage:**
    ```bash
    setup-devbox version
    ```
- #### `now` Installs and Configures Tools, Fonts, OS Settings, and Shell Configs.

    ##### **Description:**
    This is the primary command for provisioning and updating 
    your development environment. It orchestrates the entire 
    setup process by reading your `config.yaml` (and linked configuration files), 
    comparing them against the internal state (`state.json`), and 
    intelligently performing necessary installations and configurations. 
    It aims to be idempotent, meaning you can run it multiple times without unintended side effects.

    **Usage:**
    ```bash
    setup-devbox now [OPTIONS]
    ```
    ##### Options 
    - `--config <CONFIG>`: Optional argument to specify the path to the main configuration file (e.g., `config.yaml`). If not provided, `setup-devbox` will use the default path (typically `~/.setup-devbox/configs/config.yaml`).
    - `--state <STATE>`: Optional argument to specify a custom path for the state file (e.g., state.json). If not provided, the default state file path (typically `~/.setup-devbox/state.json`) will be used.
    - `-h`, `--help`: Print help for the now command.
    ##### Examples

    ```bash
    # Run the full setup process using default config and state paths
    setup-devbox now

    # Run the setup process using a custom main config file
    setup-devbox now --config /my/custom/path/my_main_config.yaml

    # Run the setup process using a custom state file
    setup-devbox now --state /tmp/my_devbox_state.json
    ```
- #### `generate` Generates the default configs.

  ##### **Description:**
  This command helps you get started quickly by creating boilerplate configuration files (`config.yaml`, `tools.yaml`, `settings.yaml`, `shellrc.yaml`, `fonts.yaml`)
  in a specified or default configuration directory. These generated files serve as a template, 
  which you can then customize to define your specific development environment. 
  This command will typically not overwrite existing files without explicit confirmation.

  **Usage:**
    ```bash
    setup-devbox generate [OPTIONS]
    ```
  ##### Options
  - `--config <CONFIG>`: Optional argument to specify the target directory where the main configuration file (e.g., `config.yaml`) should be generated. 
     If not provided, the default config directory (typically `~/.setup-devbox/configs/`) will be used. 
  - `--state <STATE>`: Optional argument to specify the target directory where the state file (e.g., `state.json`) should be generated. 
     If not provided, the default state file path (typically `~/.setup-devbox/state.json`) will be used. 
  - `-h`, `--help`: Print help for the generate command.
  
  ##### Examples

    ```bash
    # Generate all default configuration files in the default location
    setup-devbox generate
    
    # Generate default configurations in a custom directory
    setup-devbox generate --config /my/custom/configs/ --state /my/custom/data/state.json
    ```

- #### `sync-config` Sync or Generate configurations from state-file.

  ##### **Description:**
  This command is intended to help keep your declarative configuration files (e.g., `tools.yaml`) 
  in sync with the actual state of your system as recorded by `setup-devbox`, or to generate new configuration entries based on the current state. 
  This can be useful for:
  - **Discovery**: Creating `tools.yaml` entries for tools that `setup-devbox` has already installed via the now command.
  - **State Alignment**: Helping to align your configuration files with changes recorded in the `state.json`.

  **Usage:**
    ```bash
    setup-devbox sync-config [OPTIONS]
    ```
  ##### Options
  - `--state <STATE>`: Optional argument to specify the state file to read from. Defaults to `~/.setup-devbox/state.json`. 
  - `--output-dir <OUTPUT_DIR>`: Optional argument to specify the directory where configuration files should be generated or updated. 
    Defaults to `~/.setup-devbox/configs`. 
  - `-h`, `--help`: Print help for the sync-config command.

  ##### Examples

    ```bash
    # Sync configurations based on the default state file to the default config directory
    setup-devbox sync-config
    
    # Sync configurations from a custom state file to a custom output directory
    setup-devbox sync-config --state /my/custom/data/state.json --output-dir /my/custom/configs/
    ```