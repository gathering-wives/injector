# Injector

Extremely simple and minimal injector written in Rust that makes sure that your dependencies are loaded *before* the main executable.

## Usage

```console
$ injector --help
Usage: injector [OPTIONS]

Options:
  -c, --config-path <CONFIG_PATH>  [default: ./config.toml]
  -v, --verbose
  -h, --help                       Print help
```

## Config

Injector works by reading a config file. The config file is a TOML file with the following structure:

| Requred | Name | Description | Default |
| ------- | ---- | ---- | ----------- |
| Yes     | executable_path | Path to the executable that will be launched. | - |
| No      | current_directory | Current working directory of the process. | Directory of the executable. |
| No      | dependencies | List of dependencies to inject. | None. |
| No      | args | List of arguments to pass to the executable. | None. |

### Example config:

```toml
executable_path = 'D:\games\Wuthering Waves Game\Client\Binaries\Win64\Client-Win64-Shipping.exe'
dependencies = ['D:\dev\git\gathering-wives\dumper\target\debug\dumper.dll']
```
