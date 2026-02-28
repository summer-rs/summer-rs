## Introduction

The log plugin is a plugin built into the `spring` core module and is also the first plugin loaded by spring. This plugin does not need to be added manually by the developer.

This plugin integrates the most popular log library [`tracing`](https://tracing.rs/) and provides the following configuration items

## Configuration items

```toml
[logger]
enable = true            # Whether to enable the log function, which is enabled by default
pretty_backtrace = false # Whether to print stack information, which is disabled by default. It is recommended to enable it only during application development
level = "info"           # The default log level is info
format = "compact"       # The log format supports compact, pretty, and json, and the default is compact
time_style = "local"    # The timestamp used for log output, supporting system, uptime, local, utc, none
time_pattern = "%Y-%m-%dT%H:%M:%S"  # The output format of the timestamp, effective only when time_style is local or utc
with_fields = [                     # Carry other fields when outputting logs, the following fields are not carried by default
"thread_id",                        # Current thread ID
"thread_name",                      # Current thread name
"file",                             # Output the source file name of the log
"line_number",                      # Output the line number of the source file of the log
"internal_errors",                  # Whether to output the internal errors contained when an error occurs
]
override_filter = "info,axum=debug" # Override the default log filter level, and specify the log level for the crate library
file = { enable = true } # Whether to write logs to files, which is not enabled by default
```

> The [time_pattern](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/time/index.html) here is implemented using [chrono](https://docs.rs/chrono). If you need to customize it, you can refer to [chrono's formatting syntax](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

You can also configure the log file in more detail
```toml
[logger.file]
enable = true      # Whether to write logs to files
non_blocking = true # Whether to enable non-blocking writing, which is enabled by default
format = "compact"  # Log format supports compact, pretty, json, the default is compact
rotation = "daily"  # Log rotation mode minutely, hourly, daily, never, the default is daily rotation
dir = "./logs"      # Log file directory
filename_prefix = "app" # Log file prefix name
filename_suffix = "log" # Log file suffix name
max_log_files = 365     # Maximum number of logs to retain
```