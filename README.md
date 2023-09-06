# extension-version-watcher

rust program to check for updates in chrome extensions

## installation and usage

simply git clone then `cargo build --release` or `cargo install --path .`. alternatively, `cargo install --git https://github.com/staticallyamazing/extension-version-watcher`. there are currently no
prebuilt binaries and it is not on crates.io.

diff generation currently requires [prettier](https://prettier.io) to be installed and available on `PATH`. if you would like to use a custom prettier config, simply create .prettierrc.json in the
current working directory. extension-version-watcher will see this and use it instead of the builtin config.

this program has no command line flags. it can be configured by the `config.toml` file. [the example config file](./config.example.toml) will be automatically written to `config.toml` if it does not
already exist. please see the [the example config file](./config.example.toml) for all available configuration options and descriptions on what they do.
