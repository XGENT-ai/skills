Return to top

# mise.toml reference [​](#mise-toml-reference)

Every key mise reads from a `mise.toml`: its type, what it does and where it goes, generated from [`schema/mise.json`](https://mise.jdx.dev/schema/mise.json) so it matches this version of mise.

`/`

37 sections Expand all Collapse all

Tools

- `tools`289
- `tool_alias`5
- `tool_config`1
- `plugins`1
- `wrappers`5

Environment

- `env`34
- `vars`15
- `redactions`
- `secrets`2

Tasks

- `tasks`98
- `task_config`18
- `task_templates`42
- `monorepo`15
- `monorepo_root`

Hooks and shell

- `hooks`17
- `watch_files`4
- `shell_alias`1

Config file

- `settings`
- `include`
- `min_version`2
- `doctor`10
- `_`

Project services

- `deps`27
- `oci`16
- `daemons`35
- `daemon_groups`2
- `daemons_settings`2
- `daemon_providers`15

Machine setup

- `bootstrap`419
- `dotfiles`30
- `dotfile_groups`35
- `history`10

Deprecated

- `alias`→
- `dotenv`
- `env_file`
- `env_path`
- `experimental_monorepo_root`

- `<name>` stands for a name you choose, such as a task's name in `tasks.<name>`.
- `string | string[]` means either form works.
- `/` filters; `↑` `↓` `←``→` move through the tree.

- How the sections fit together: [mise.toml](https://mise.jdx.dev/configuration.html)
- Explain your own file: [`mise config explain`](https://mise.jdx.dev/cli/config/explain.html) or the [playground](https://mise.jdx.dev/configuration/playground.html)
- For scripts and agents: [reference.md](https://mise.jdx.dev/configuration/reference.md)

## Tools

[`tools`](#tools)289

dev tools to use

[Dev tools guide →](https://mise.jdx.dev/dev-tools/)

[`tool_alias`](#tool_alias)5

Tool version aliases

[Tool aliases guide →](https://mise.jdx.dev/dev-tools/aliases.html)

[`tool_config`](#tool_config)1

policy for tools declared by this config root

[Tool policy guide →](https://mise.jdx.dev/configuration.html#tool-config)

[`plugins`](#plugins)1

plugins to use

[Plugins guide →](https://mise.jdx.dev/configuration.html#plugins)

[`wrappers`](#wrappers)5

commands that intercept a binary name before delegating to the active toolset

[Command wrappers guide →](https://mise.jdx.dev/dev-tools/shims.html#command-wrappers)

## Environment

[`env`](#env)34

environment variables to set

[Environment variables guide →](https://mise.jdx.dev/environments/)

[`vars`](#vars)15

variables to set for use in config templates

[Config variables guide →](https://mise.jdx.dev/configuration/vars.html)

[`redactions`](#redactions)

env or vars keys to redact from logs

[Redaction guide →](https://mise.jdx.dev/environments/secrets/#redaction)

[`secrets`](#secrets)experimental2

Sources for mise secrets: values resolved only when mise starts a task or mise x command that was granted them

[fnox guide →](https://mise.jdx.dev/environments/secrets/fnox.html)

## Tasks

[`tasks`](#tasks)98

task runner tasks

[Tasks guide →](https://mise.jdx.dev/tasks/)

[`task_config`](#task_config)18

configuration for task execution and management

[Task configuration guide →](https://mise.jdx.dev/tasks/task-configuration.html#task-config-options)

[`task_templates`](#task_templates)42

task templates that can be extended by tasks via extends

[Task templates guide →](https://mise.jdx.dev/tasks/templates.html)

[`monorepo`](#monorepo)15

configuration for monorepo task discovery

[Monorepo tasks guide →](https://mise.jdx.dev/tasks/monorepo.html)

[`monorepo_root`](#monorepo_root)

marks this config as a monorepo root for task path syntax

[Monorepo tasks guide →](https://mise.jdx.dev/tasks/monorepo.html)

## Hooks and shell

[`hooks`](#hooks)17

hooks to run on events like cd, enter, leave

[Hooks guide →](https://mise.jdx.dev/hooks.html)

[`watch_files`](#watch_files)4

files to watch for changes and run scripts when modified

[Hooks guide →](https://mise.jdx.dev/hooks.html)

[`shell_alias`](#shell_alias)1

shell aliases

[Shell aliases guide →](https://mise.jdx.dev/shell-aliases.html)

## Config file

[`settings`](#settings)

mise settings

[Settings guide →](https://mise.jdx.dev/configuration/settings.html)

[`include`](#include)

config files merged into this one, ranking just below it: a local path (relative to this file), git::\<url>//\<path>.toml?ref=\<ref> or oci::\<registry>/\<repo>\[:tag|@sha256:\<digest>] (the artifact holds a mise.toml)

[include guide →](https://mise.jdx.dev/configuration.html#include)

[`min_version`](#min_version)2

minimum version of mise required to use this config

[Minimum mise version guide →](https://mise.jdx.dev/configuration.html#minimum-mise-version)

[`doctor`](#doctor)10

Project diagnostic checks, run explicitly with mise doctor project

[Project diagnostics guide →](https://mise.jdx.dev/configuration/project-diagnostics.html)

[`_`](#_)

free-form data for your own use

[Free-form data guide →](https://mise.jdx.dev/configuration.html#free-form-data)

## Project services

[`deps`](#deps)experimental27

configure deps providers

[Project dependencies guide →](https://mise.jdx.dev/dev-tools/deps.html)

[`oci`](#oci)experimental16

configuration for mise oci build

[OCI images guide →](https://mise.jdx.dev/dev-tools/mise-oci.html)

[`daemons`](#daemons)experimental35

Experimental project daemons: custom pitchfork commands or managed presets

[Daemons guide →](https://mise.jdx.dev/daemons.html)

[`daemon_groups`](#daemon_groups)experimental2

Experimental named groups of project daemons

[Daemon groups guide →](https://mise.jdx.dev/daemons.html#groups)

[`daemons_settings`](#daemons_settings)experimental2

Experimental project-wide daemon options

[Ports, URLs, and worktrees guide →](https://mise.jdx.dev/daemons/worktrees.html#namespaces)

[`daemon_providers`](#daemon_providers)experimental15

Experimental shared servers owned by the user

[Share daemons across projects guide →](https://mise.jdx.dev/daemons/sharing.html)

## Machine setup

[`bootstrap`](#bootstrap)419

machine-global bootstrapping (system packages, repos, macOS defaults, launchd agents, login shell)

[Bootstrap guide →](https://mise.jdx.dev/bootstrap.html)

[`dotfiles`](#dotfiles)30

dotfiles applied with mise dotfiles apply or mise bootstrap, keyed by target path or a logical name when every variant overrides the target

[Dotfiles guide →](https://mise.jdx.dev/dotfiles.html)

[`dotfile_groups`](#dotfile_groups)35

named directory trees of dotfiles, applied with mise dotfiles apply or mise bootstrap while \[bootstrap] dotfile\_groups selects them (or is unset)

[Dotfile groups guide →](https://mise.jdx.dev/dotfiles/groups.html)

[`history`](#history)10

dotfiles history: what is never captured, reload commands, and the setup repository (https://mise.jdx.dev/history.html)

[Dotfiles history guide →](https://mise.jdx.dev/dotfiles/history.html)

## Deprecated

[`alias`](#alias)

custom shorthands

[Tool aliases guide →](https://mise.jdx.dev/dev-tools/aliases.html)

[`dotenv`](#dotenv)

dotenv file(s) to load

[Environment variables guide →](https://mise.jdx.dev/environments/)

[`env_file`](#env_file)

dotenv file(s) to load

[Environment variables guide →](https://mise.jdx.dev/environments/)

[`env_path`](#env_path)

PATH entries to add

[Environment variables guide →](https://mise.jdx.dev/environments/)

[`experimental_monorepo_root`](#experimental_monorepo_root)

deprecated alias of monorepo\_root

[Monorepo tasks guide →](https://mise.jdx.dev/tasks/monorepo.html)

## `_`

free-form data for your own use; mise never reads it

## `alias`

custom shorthands

## `bootstrap`

machine-global bootstrapping (system packages, repos, macOS defaults, launchd agents, login shell)

### `bootstrap.files`

Managed system files keyed by absolute target path

### `bootstrap.files.<key>`

file whose content, owner, group, and mode mise manages, keyed by an absolute or \~/ target path

### `bootstrap.files.<key>.phase`

When to apply the managed path during bootstrap; pre-packages runs before the pre-packages hook

### `bootstrap.files.<key>.source`

file to copy the content from; a relative path resolves from the declaring config's directory and \~/ from your home directory

### `bootstrap.files.<key>.content`

inline file content; cannot be combined with source

### `bootstrap.files.<key>.owner`

user that should own the file; left as it is when unset

### `bootstrap.files.<key>.group`

group that should own the file; left as it is when unset

### `bootstrap.files.<key>.mode`

octal permissions such as "0640"; defaults to "0644" for a file with source or content, otherwise left as it is

### `bootstrap.files.<key>.template`

render source or content with mise's template engine; without it, template syntax is written unchanged

### `bootstrap.files.<key>.remove_empty`

remove the target when the template renders to empty or whitespace-only content; requires template = true

### `bootstrap.files.<key>.state`

present creates or updates the file; absent removes it whatever its content

### `bootstrap.files.<key>.replace`

replace a path of another type at the target, such as an empty directory; requires source or content

### `bootstrap.files.<key>.notify`

system services declared in \[bootstrap.services] to reload or restart when mise changes this file

### `bootstrap.files.<key>.os`

OS or OS/arch selectors; entries on other platforms are skipped

### `bootstrap.directories`

Managed system directories keyed by absolute target path

### `bootstrap.directories.<key>`

directory whose owner, group, and mode mise manages, keyed by an absolute or \~/ target path

### `bootstrap.directories.<key>.phase`

When to apply the managed path during bootstrap; pre-packages runs before the pre-packages hook

### `bootstrap.directories.<key>.owner`

user that should own the directory; left as it is when unset, and parents mise creates get the system defaults

### `bootstrap.directories.<key>.group`

group that should own the directory; left as it is when unset, and parents mise creates get the system defaults

### `bootstrap.directories.<key>.mode`

octal permissions such as "0755"; defaults to "0755"

### `bootstrap.directories.<key>.state`

present creates the directory with any missing parents; absent removes it, which must then be empty unless recursive is set

### `bootstrap.directories.<key>.recursive`

with state = "absent", delete the directory and everything in it

### `bootstrap.directories.<key>.replace`

replace a path of another type at the target, such as a file

### `bootstrap.directories.<key>.notify`

system services declared in \[bootstrap.services] to reload or restart when mise changes this directory

### `bootstrap.directories.<key>.os`

OS or OS/arch selectors; entries on other platforms are skipped

### `bootstrap.dotfile_groups`

the dotfile groups this machine applies; when unset, every group applies. Entries without a group always apply, and a more local config's list replaces the others

### `bootstrap.config_roots`

deprecated and will be removed in mise 2027.3.3; move each selected root into a conf.d folder instead (<https://mise.jdx.dev/configuration.html#conf-d-folders>)

### `bootstrap.remote`

OpenSSH targets used by `mise bootstrap remote`

### `bootstrap.remote.source`

local project directory sent to inventory hosts

### `bootstrap.remote.mise_env`

default ordered config environments loaded by remote bootstrap

### `bootstrap.remote.install_mise`

install the provisioned mise on inventory hosts instead of only staging it

### `bootstrap.remote.copy_links`

dereference every symbolic link in the source archive

### `bootstrap.remote.copy_link`

source-relative symbolic links to dereference

### `bootstrap.remote.exclude`

additional source archive exclusion patterns

### `bootstrap.remote.hosts`

remote bootstrap inventory keyed by target name

### `bootstrap.remote.hosts.<key>`

SSH target that `mise bootstrap remote` can bootstrap, selected by name or tag

### `bootstrap.remote.hosts.<key>.host`

SSH host name or address

### `bootstrap.remote.hosts.<key>.user`

SSH user

### `bootstrap.remote.hosts.<key>.port`

SSH port

### `bootstrap.remote.hosts.<key>.identity_file`

SSH identity file

### `bootstrap.remote.hosts.<key>.source`

host-specific local project directory to send

### `bootstrap.remote.hosts.<key>.mise_env`

ordered config environments loaded by remote bootstrap

### `bootstrap.remote.hosts.<key>.copy_links`

dereference every symbolic link in this host's source archive

### `bootstrap.remote.hosts.<key>.copy_link`

source-relative symbolic links to dereference for this host

### `bootstrap.remote.hosts.<key>.exclude`

additional source archive exclusion patterns for this host

### `bootstrap.remote.hosts.<key>.ssh_options`

OpenSSH options passed with `-o`

### `bootstrap.remote.hosts.<key>.tags`

inventory selection tags

### `bootstrap.remote.hosts.<key>.install_mise`

install the provisioned mise on this host instead of only staging it

### `bootstrap.remote.hosts.<key>.mise_bin`

local mise executable to upload

### `bootstrap.remote.hosts.<key>.remote_mise`

existing mise executable name or path on the host

### `bootstrap.remote.hosts.<key>.bootstrap_command`

remote shell command that installs mise

### `bootstrap.services`

System services and portable user services managed by mise bootstrap, keyed by service name

### `bootstrap.services.<key>`

Linux systemd unit that mise starts, stops, enables, or masks (a system unit, or with scope user an existing user unit), or a user service mise defines from command or builtin

### `bootstrap.services.<key>.state`

running or stopped; absent removes a user service's definition (user scope only)

### `bootstrap.services.<key>.enabled`

start the service at boot (system scope) or at login (user scope)

### `bootstrap.services.<key>.masked`

System scope only

### `bootstrap.services.<key>.on_change`

System scope only

### `bootstrap.services.<key>.scope`

Defaults to system; builtin implies user

### `bootstrap.services.<key>.builtin`

A mise-provided user service

### `bootstrap.services.<key>.command`

User service command; mutually exclusive with builtin

### `bootstrap.services.<key>.description`

text the service manager shows for the service; user scope only

### `bootstrap.services.<key>.restart`

when to restart the process after it exits: on-failure, always, or never; on Windows, always and on-failure both retry failed runs only; user scope only

### `bootstrap.services.<key>.environment`

environment variables for the process; user scope only

### `bootstrap.services.<key>.environment.<key>`

value of the environment variable

### `bootstrap.services.<key>.working_directory`

directory to run the process in, with \~ expanded; user scope only

### `bootstrap.services.<key>.requires_tools`

start the service only after `mise bootstrap` installs \[tools] and plugin packages; user scope only

### `bootstrap.compose`

Docker Compose projects managed with `mise bootstrap compose`, keyed by project name. Text fields support Tera templates.

### `bootstrap.compose.<key>`

Docker Compose project that mise brings up, stops, or removes

### `bootstrap.compose.<key>.project_dir`

absolute directory containing the Compose project

### `bootstrap.compose.<key>.files`

Compose file paths relative to project\_dir unless absolute

### `bootstrap.compose.<key>.env_files`

environment file paths relative to project\_dir unless absolute

### `bootstrap.compose.<key>.project_name`

optional Compose project name

### `bootstrap.compose.<key>.profiles`

Compose profiles to enable, passed with --profile

### `bootstrap.compose.<key>.services`

services to manage; defaults to every service the files and profiles enable

### `bootstrap.compose.<key>.oneshot`

services that run once and count as converged after exiting with code 0; when services is set, they must be listed there too

### `bootstrap.compose.<key>.state`

running (compose up), stopped (compose stop, keeping containers and data), or absent (compose down)

### `bootstrap.compose.<key>.pull`

image pull policy passed to compose up as --pull

### `bootstrap.compose.<key>.build`

auto lets Compose decide, always passes --build, never passes --no-build

### `bootstrap.compose.<key>.recreate`

auto lets Compose decide, always passes --force-recreate, never passes --no-recreate

### `bootstrap.compose.<key>.wait`

pass --wait to compose up, so it waits for services to be running and healthy

### `bootstrap.compose.<key>.wait_timeout`

wait timeout in seconds

### `bootstrap.compose.<key>.timeout`

Compose shutdown timeout in seconds

### `bootstrap.compose.<key>.remove_orphans`

remove containers for services that are no longer in the Compose model

### `bootstrap.compose.<key>.renew_anonymous_volumes`

pass --renew-anon-volumes to compose up, recreating anonymous volumes

### `bootstrap.compose.<key>.down_volumes`

pass --volumes to compose down, removing the project's volumes

### `bootstrap.compose.<key>.down_images`

images compose down removes, passed as --rmi

### `bootstrap.compose.<key>.sudo`

run every Compose and engine command through sudo, for a project on the system Docker daemon

### `bootstrap.compose.<key>.command`

standalone Compose command

### `bootstrap.compose.<key>.engine_command`

container engine command

### `bootstrap.compose.<key>.depends_on`

bootstrap resource dependencies

### `bootstrap.plugins`

package manager plugins to install with `mise bootstrap plugins apply`, keyed by manager name

### `bootstrap.plugins.<name>`

plugin git URL

### `bootstrap.packages`

system packages to install with `mise bootstrap packages apply`, keyed by "manager:package" (e.g. "apt:libssl-dev", "brew:postgresql@17", "dnf:openssl-devel", "pacman:base-devel")

### `bootstrap.packages.<key>`

version pin in the manager's native format, or "latest"

### `bootstrap.packages.<key>.version`

version pin in the manager's native format, or "latest"

### `bootstrap.packages.<key>.os`

operating system or OS/architecture selectors

### `bootstrap.packages.<key>.env`

mise environments in which this package is enabled

### `bootstrap.packages.<key>.adopt`

adopt an existing identical app artifact instead of replacing it (brew-cask and macos-app)

### `bootstrap.packages.<key>.appdir`

brew-cask only: application directory for this cask and its cask dependencies; takes precedence over MISE\_BREW\_CASK\_OPT\_APPDIR

### `bootstrap.packages.<key>.url`

macos-app only: download URL for the app archive; {{version}} is replaced with the declared version

### `bootstrap.packages.<key>.sha256`

macos-app only: expected sha256 of the download as 64 hex characters; required with url. Homebrew's no\_check sentinel is not accepted

### `bootstrap.packages.<key>.artifact`

macos-app only: app bundle to install from the archive, e.g. Nuvio.app; required with url

### `bootstrap.packages.<key>.channel`

snap only: channel to install from and track, e.g. latest/stable or 22/edge

### `bootstrap.packages.<key>.classic`

snap only: install with classic confinement

### `bootstrap.packages.<key>.state`

desired package state; pacman, scoop, snap, and zypper support absent

### `bootstrap.repos`

git repositories to clone or update with `mise bootstrap repos apply`, keyed by target path

### `bootstrap.repos.<key>`

git repository to clone to this path, then keep on ref when one is set

### `bootstrap.repos.<key>.url`

git repository URL

### `bootstrap.repos.<key>.ref`

optional branch, tag, or full commit SHA to check out

### `bootstrap.repos.<key>.filter`

partial clone filter passed to `git clone --filter`, such as `blob:none`; applies to the first clone only

### `bootstrap.repos.<key>.depth`

shallow clone depth passed to `git clone --depth`; applies to the first clone only

### `bootstrap.repos.<key>.submodules`

clone with `--recurse-submodules`; applies to the first clone only

### `bootstrap.macos`

macOS-specific bootstrap config

### `bootstrap.macos.dock`

curated Dock preferences that compile into macOS defaults

### `bootstrap.macos.dock.apps`

Pinned application paths in order; absolute or \~/ paths ending in .app. An empty array removes application tiles.

### `bootstrap.macos.dock.autohide_delay`

delay before showing the Dock

### `bootstrap.macos.dock.autohide_time_modifier`

Dock autohide animation time modifier

### `bootstrap.macos.dock.autohide`

hide and show the Dock automatically

### `bootstrap.macos.dock.orientation`

Dock screen edge

### `bootstrap.macos.dock.tilesize`

Dock icon size

### `bootstrap.macos.dock.magnification`

enable Dock magnification

### `bootstrap.macos.dock.largesize`

Dock magnified icon size

### `bootstrap.macos.dock.show_recents`

show recent applications in the Dock

### `bootstrap.macos.dock.mru_spaces`

automatically rearrange Spaces based on most recent use

### `bootstrap.macos.finder`

curated Finder preferences that compile into macOS defaults

### `bootstrap.macos.finder.sort_folders_first`

keep folders first when sorting by name

### `bootstrap.macos.finder.save_new_documents_to_cloud`

save new documents to iCloud by default (applies globally to application save dialogs)

### `bootstrap.macos.finder.show_all_files`

show hidden files in Finder

### `bootstrap.macos.finder.show_pathbar`

show the Finder path bar

### `bootstrap.macos.finder.show_status_bar`

show the Finder status bar

### `bootstrap.macos.finder.show_extensions_warning`

show the warning when changing file extensions

### `bootstrap.macos.finder.preferred_view_style`

Finder preferred view style

### `bootstrap.macos.keyboard`

curated keyboard preferences that compile into macOS defaults

### `bootstrap.macos.keyboard.automatic_capitalization`

automatically capitalize words

### `bootstrap.macos.keyboard.automatic_spelling_correction`

automatically correct spelling

### `bootstrap.macos.keyboard.key_repeat`

keyboard repeat interval

### `bootstrap.macos.keyboard.initial_key_repeat`

delay before key repeat starts

### `bootstrap.macos.keyboard.press_and_hold`

enable press-and-hold accent picker

### `bootstrap.macos.keyboard.fn_state`

use F1, F2, etc. as standard function keys

### `bootstrap.macos.trackpad`

curated trackpad preferences that compile into macOS defaults

### `bootstrap.macos.trackpad.tap_to_click`

enable tap to click for built-in and Bluetooth trackpads

### `bootstrap.macos.trackpad.three_finger_drag`

enable three finger drag for built-in and Bluetooth trackpads

### `bootstrap.macos.defaults`

macOS user defaults to apply with `mise bootstrap macos defaults apply`, keyed by preferences domain (e.g. "com.apple.dock", "NSGlobalDomain")

### `bootstrap.macos.defaults.<key>`

defaults keys and their desired values for this domain

### `bootstrap.macos.defaults.<key>.<key>`

desired value, written with the matching `defaults write` type; arrays and tables become property list arrays and dictionaries

### `bootstrap.macos.defaults.<key>.<key>.<key>`

a property list value: boolean, integer, real, string, or an array or dictionary of these

### `bootstrap.macos.defaults.<key>.<key>.<key>.<key>`

a property list value: boolean, integer, real, string, or an array or dictionary of these

### `bootstrap.macos.defaults.<key>.<key>.<key>.<key>.<key>`

a property list value: boolean, integer, real, string, or an array or dictionary of these

### `bootstrap.macos.defaults.<key>.<key>.<key>.<key>.<key>.<key>`

a property list value: boolean, integer, real, string, or an array or dictionary of these

### `bootstrap.macos.defaults_entries`

Explicit macOS preferences with a selectable host scope

### `bootstrap.macos.defaults_entries[].domain`

preferences domain, such as "com.apple.dock" or "NSGlobalDomain"; -g and -globalDomain mean NSGlobalDomain

### `bootstrap.macos.defaults_entries[].key`

preference key within the domain

### `bootstrap.macos.defaults_entries[].host`

any writes the preference for every host; current writes it for this machine only, like defaults -currentHost

### `bootstrap.macos.defaults_entries[].path`

Dictionary keys below the preference key; omitted to replace the whole value

### `bootstrap.macos.defaults_entries[].value`

desired value, written with the matching property list type; replaces the selected value in full

### `bootstrap.macos.launchd`

macOS launchd bootstrap config

### `bootstrap.macos.launchd.agents`

macOS user LaunchAgents to write and load with `mise bootstrap macos launchd-agents apply`

### `bootstrap.macos.launchd.agents.<key>`

macOS LaunchAgent written to \~/Library/LaunchAgents/dev.mise.\<name>.plist and loaded for the current user

### `bootstrap.macos.launchd.agents.<key>.program`

program path for ProgramArguments\[0]

### `bootstrap.macos.launchd.agents.<key>.args`

additional ProgramArguments entries

### `bootstrap.macos.launchd.agents.<key>.run_at_load`

write RunAtLoad

### `bootstrap.macos.launchd.agents.<key>.keep_alive`

write KeepAlive

### `bootstrap.macos.launchd.agents.<key>.keep_alive_on_failure`

write KeepAlive = { SuccessfulExit = false }, so launchd relaunches the job only after a failure; cannot be combined with keep\_alive

### `bootstrap.macos.launchd.agents.<key>.start_interval`

write StartInterval in seconds

### `bootstrap.macos.launchd.agents.<key>.throttle_interval`

write ThrottleInterval in seconds

### `bootstrap.macos.launchd.agents.<key>.nice`

write Nice, the niceness of the process, from -20 to 20

### `bootstrap.macos.launchd.agents.<key>.process_type`

write ProcessType, the scheduling band launchd runs the job in

### `bootstrap.macos.launchd.agents.<key>.start_calendar_interval`

write one or more StartCalendarInterval schedules

### `bootstrap.macos.launchd.agents.<key>.start_calendar_interval.minute`

minute of the hour, 0-59; every minute when unset

### `bootstrap.macos.launchd.agents.<key>.start_calendar_interval.hour`

hour of the day, 0-23; every hour when unset

### `bootstrap.macos.launchd.agents.<key>.start_calendar_interval.day`

day of the month, 1-31; every day when unset

### `bootstrap.macos.launchd.agents.<key>.start_calendar_interval.weekday`

day of the week, 0-7, where 0 and 7 are Sunday; every day when unset

### `bootstrap.macos.launchd.agents.<key>.start_calendar_interval.month`

month of the year, 1-12; every month when unset

### `bootstrap.macos.launchd.agents.<key>.queue_directories`

write QueueDirectories (absolute paths; `~` and `~/` are expanded)

### `bootstrap.macos.launchd.agents.<key>.launch_events`

write LaunchEvents, keyed by event stream, then event name, with launchd's case-sensitive property names

### `bootstrap.macos.launchd.agents.<key>.launch_events.<key>`

### `bootstrap.macos.launchd.agents.<key>.launch_events.<key>.<key>`

### `bootstrap.macos.launchd.agents.<key>.launch_events.<key>.<key>.<key>`

a property list value: boolean, integer, real, string, or an array or dictionary of these

### `bootstrap.macos.launchd.agents.<key>.environment`

environment variables for EnvironmentVariables

### `bootstrap.macos.launchd.agents.<key>.environment.<key>`

value of the environment variable

### `bootstrap.macos.launchd.agents.<key>.working_directory`

write WorkingDirectory

### `bootstrap.macos.launchd.agents.<key>.stdout_path`

write StandardOutPath

### `bootstrap.macos.launchd.agents.<key>.stderr_path`

write StandardErrorPath

### `bootstrap.macos.launchd.agents.<key>.kickstart`

run `launchctl kickstart -k` after loading the agent

### `bootstrap.macos.login_items`

applications to open at login (System Settings > General > Login Items & Extensions)

### `bootstrap.macos.login_items.apps`

application bundles to open at login: absolute or `~/` paths ending in `.app`

### `bootstrap.macos.login_items.exclusive`

also remove login items that are not declared (default: false, so apps that register themselves are left alone)

### `bootstrap.linux`

Linux-specific bootstrap config

### `bootstrap.linux.firewall`

Linux host firewall policy and rules managed with `mise bootstrap firewall`

### `bootstrap.linux.firewall.backend`

firewall backend; auto reuses managed or active host infrastructure

### `bootstrap.linux.firewall.state`

desired lifecycle of mise-managed firewall state

### `bootstrap.linux.firewall.default_incoming`

default action for incoming traffic

### `bootstrap.linux.firewall.default_outgoing`

default action for outgoing traffic

### `bootstrap.linux.firewall.exclusive`

treat the declared rules as the complete owned ruleset; UFW reset also removes unrelated UFW rules

### `bootstrap.linux.firewall.allow_lockout`

allow default-deny or reject over SSH without a rule covering the connected peer

### `bootstrap.linux.firewall.rules`

named firewall rules reconciled without deleting undeclared managed rules

### `bootstrap.linux.firewall.rules[].name`

stable managed rule identifier

### `bootstrap.linux.firewall.rules[].state`

present adds the rule; absent removes a rule mise added earlier

### `bootstrap.linux.firewall.rules[].direction`

traffic direction the rule matches

### `bootstrap.linux.firewall.rules[].action`

what to do with matching traffic; limit needs incoming TCP and is not available on firewalld

### `bootstrap.linux.firewall.rules[].port`

single port number or inclusive range

### `bootstrap.linux.firewall.rules[].protocol`

protocol to match; any when unset, and UFW supports only tcp and udp

### `bootstrap.linux.firewall.rules[].source`

source IPv4 or IPv6 CIDR network

### `bootstrap.linux.firewall.rules[].destination`

destination IPv4 or IPv6 CIDR network

### `bootstrap.linux.firewall.rules[].interface`

network interface (nftables and UFW only)

### `bootstrap.linux.systemd`

systemd user service and timer bootstrap config

### `bootstrap.linux.systemd.units`

systemd user services and timers to write and start with `mise bootstrap linux systemd-units apply`

### `bootstrap.linux.systemd.units.<key>`

systemd user unit written to \~/.config/systemd/user/dev.mise.\<name>.service, or .timer when it sets a timer key

### `bootstrap.linux.systemd.units.<key>.description`

write Description in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.after`

write After dependencies in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.wants`

write Wants dependencies in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.requires`

write Requires dependencies in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.before`

write Before ordering in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.binds_to`

write BindsTo dependencies in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.part_of`

write PartOf dependencies in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.conflicts`

write Conflicts dependencies in the \[Unit] section

### `bootstrap.linux.systemd.units.<key>.exec_start_pre`

write one ExecStartPre entry per command in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.exec_start`

write ExecStart in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.exec_start_post`

write one ExecStartPost entry per command in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.type`

write Type in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.remain_after_exit`

write RemainAfterExit in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.exec_stop`

write ExecStop in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.exec_stop_post`

write one ExecStopPost entry per command in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.timeout_start_sec`

write TimeoutStartSec in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.timeout_stop_sec`

write TimeoutStopSec in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.no_new_privileges`

write NoNewPrivileges in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.private_tmp`

write PrivateTmp in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.environment`

environment variables for Environment entries

### `bootstrap.linux.systemd.units.<key>.environment.<key>`

value of the environment variable

### `bootstrap.linux.systemd.units.<key>.environment_file`

write one EnvironmentFile entry per path in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.nice`

write Nice in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.umask`

write UMask in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.working_directory`

write WorkingDirectory in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.restart`

write Restart in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.restart_sec`

write RestartSec in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.standard_output`

write StandardOutput in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.standard_error`

write StandardError in the \[Service] section

### `bootstrap.linux.systemd.units.<key>.on_boot_sec`

write OnBootSec in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.on_unit_active_sec`

write OnUnitActiveSec in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.on_unit_inactive_sec`

write OnUnitInactiveSec in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.on_calendar`

write OnCalendar in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.randomized_delay_sec`

write RandomizedDelaySec in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.accuracy_sec`

write AccuracySec in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.persistent`

write Persistent in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.unit`

write Unit in the \[Timer] section

### `bootstrap.linux.systemd.units.<key>.start`

restart the unit after writing it; when false, stop the unit after writing it

### `bootstrap.linux.systemd.units.<key>.wanted_by`

write WantedBy entries in the \[Install] section and enable the unit; an empty array disables previous enablement

### `bootstrap.linux.systemd.units.<key>.state`

present writes and manages the unit; absent stops, disables, and removes both dev.mise.\<name>.service and dev.mise.\<name>.timer

### `bootstrap.user`

current-user bootstrap settings

### `bootstrap.user.login_shell`

current user's desired login shell, applied with `chsh -s`; an absolute path or a bare shell name resolved after packages are installed; may be a template

### `bootstrap.apt`

apt-specific bootstrap package config

### `bootstrap.apt.sources`

apt repositories added before apt packages are installed, keyed by a name used in the files mise writes

### `bootstrap.apt.sources.<key>`

apt source: a Launchpad PPA, or repository URIs with the key that signs them

### `bootstrap.apt.sources.<key>.ppa`

Launchpad PPA as owner/archive, such as git-core/ppa; added on Ubuntu and Ubuntu-based releases only

### `bootstrap.apt.sources.<key>.uris`

repository URL(s), space-separated

### `bootstrap.apt.sources.<key>.suites`

suite(s), space-separated; defaults to the release codename

### `bootstrap.apt.sources.<key>.components`

component(s), space-separated; defaults to main, or none when suites is a path ending in /

### `bootstrap.apt.sources.<key>.architectures`

architecture(s) to fetch, space-separated

### `bootstrap.apt.sources.<key>.key`

http(s) URL of the repository's signing key, armored or binary

### `bootstrap.apt.sources.<key>.fingerprint`

full primary key fingerprint(s) the signing key must have; required with key

### `bootstrap.dnf`

dnf-specific bootstrap package config

### `bootstrap.dnf.repos`

dnf repositories enabled before dnf packages are installed, keyed by a name for the entry

### `bootstrap.dnf.repos.<key>`

dnf repository to enable, given as a COPR project

### `bootstrap.dnf.repos.<key>.copr`

COPR project as owner/project or @group/project, such as scottames/ghostty; enabled as dnf copr enable would

### `bootstrap.brew`

Homebrew-specific bootstrap package config

### `bootstrap.brew.adopt`

adopt existing app artifacts by default for configured brew-cask packages

### `bootstrap.brew.homebrew_metadata`

write Homebrew's cask metadata after mise installs a brew-cask package, so `brew upgrade` and `brew outdated` manage it afterwards

### `bootstrap.brew.taps`

Homebrew tap names mapped to custom git URLs

### `bootstrap.brew.taps.<key>`

git URL of the tap's GitHub repository, used instead of [https://github.com/\<owner](https://github.com/&lt;owner)>/homebrew-\<tap>.git

### `bootstrap.hooks`

commands to run before and after bootstrap phases

### `bootstrap.hooks.pre-packages`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre-packages.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre-packages.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre-packages.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre_packages`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre_packages.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre_packages.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre_packages.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post-packages`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post-packages.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post-packages.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post-packages.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post_packages`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post_packages.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post_packages.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post_packages.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre-repos`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre-repos.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre-repos.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre-repos.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre_repos`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre_repos.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre_repos.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre_repos.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post-repos`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post-repos.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post-repos.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post-repos.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post_repos`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post_repos.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post_repos.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post_repos.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre-dotfiles`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre-dotfiles.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre-dotfiles.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre-dotfiles.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre_dotfiles`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre_dotfiles.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre_dotfiles.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre_dotfiles.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post-dotfiles`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post-dotfiles.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post-dotfiles.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post-dotfiles.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post_dotfiles`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post_dotfiles.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post_dotfiles.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post_dotfiles.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre-defaults`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre-defaults.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre-defaults.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre-defaults.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre_defaults`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre_defaults.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre_defaults.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre_defaults.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post-defaults`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post-defaults.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post-defaults.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post-defaults.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post_defaults`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post_defaults.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post_defaults.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post_defaults.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre-user`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre-user.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre-user.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre-user.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre_user`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre_user.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre_user.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre_user.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post-user`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post-user.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post-user.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post-user.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post_user`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post_user.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post_user.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post_user.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre-tools`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre-tools.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre-tools.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre-tools.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.pre_tools`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.pre_tools.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.pre_tools.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.pre_tools.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post-tools`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post-tools.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post-tools.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post-tools.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.post_tools`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.post_tools.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.post_tools.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.post_tools.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.hooks.final`

command or commands to run for a bootstrap phase

### `bootstrap.hooks.final.run`

shell command or commands to run in order, with the default inline shell; Tera templates are rendered first

### `bootstrap.hooks.final.run_windows`

shell command or commands to run instead of `run` on Windows; set alone, the hook does nothing on other platforms

### `bootstrap.hooks.final.description`

what `mise bootstrap` prints instead of the command; `--verbose` and `--dry-run` print the command

### `bootstrap.mise_shell_activate`

declarative mise shell activation snippets, applied with `mise bootstrap mise-shell-activate apply`

### `bootstrap.mise_shell_activate.bash`

manage the activation blocks in \~/.bash\_profile (shims) and \~/.bashrc (activate); a mode applies to both files

### `bootstrap.mise_shell_activate.bash.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.bash.mode`

activation mode; defaults according to the target

### `bootstrap.mise_shell_activate.zsh`

manage the activation blocks in \~/.zprofile (shims) and \~/.zshrc (activate); a mode applies to both files, and \~/.zshenv is never written

### `bootstrap.mise_shell_activate.zsh.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.zsh.mode`

activation mode; defaults according to the target

### `bootstrap.mise_shell_activate.fish`

manage the activation block in \~/.config/fish/config.fish; activate by default, and auto activates interactive shells and uses shims otherwise

### `bootstrap.mise_shell_activate.fish.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.fish.mode`

activation mode; defaults according to the target

### `bootstrap.mise_shell_activate.bash_profile`

manage the activation block in \~/.bash\_profile; shims by default

### `bootstrap.mise_shell_activate.bash_profile.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.bash_profile.mode`

activation mode; defaults according to the target

### `bootstrap.mise_shell_activate.bashrc`

manage the activation block in \~/.bashrc; activate by default

### `bootstrap.mise_shell_activate.bashrc.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.bashrc.mode`

activation mode; defaults according to the target

### `bootstrap.mise_shell_activate.zshenv`

manage the activation block in \~/.zshenv, which zsh reads for every script; shims by default

### `bootstrap.mise_shell_activate.zshenv.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.zshenv.mode`

activation mode; defaults according to the target

### `bootstrap.mise_shell_activate.zprofile`

manage the activation block in \~/.zprofile; shims by default

### `bootstrap.mise_shell_activate.zprofile.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.zprofile.mode`

activation mode; defaults according to the target

### `bootstrap.mise_shell_activate.zshrc`

manage the activation block in \~/.zshrc; activate by default

### `bootstrap.mise_shell_activate.zshrc.enabled`

whether to manage the activation block

### `bootstrap.mise_shell_activate.zshrc.mode`

activation mode; defaults according to the target

### `bootstrap.secrets`

sensitive bootstrap inputs read from environment variables, keyed by secret name

### `bootstrap.secrets.<key>`

environment variable that provides this secret's value

### `bootstrap.secrets.<key>.env`

environment variable that provides this secret's value

### `bootstrap.secrets.<key>.description`

human-readable description shown in status output and prompts

### `bootstrap.secrets.<key>.allow_empty`

accept an empty value; empty values are rejected by default

### `bootstrap.groups`

Linux groups that bootstrap creates or removes, keyed by group name

### `bootstrap.groups.<key>`

local Linux group that bootstrap creates, updates, or removes

### `bootstrap.groups.<key>.state`

whether the group should exist

### `bootstrap.groups.<key>.gid`

numeric group ID

### `bootstrap.groups.<key>.system`

create a system group

### `bootstrap.users`

Linux users that bootstrap creates or removes, keyed by user name

### `bootstrap.users.<key>`

local Linux user that bootstrap creates, updates, or removes

### `bootstrap.users.<key>.state`

whether the user should exist

### `bootstrap.users.<key>.uid`

numeric user ID

### `bootstrap.users.<key>.group`

primary group; required for a present user

### `bootstrap.users.<key>.groups`

supplementary groups

### `bootstrap.users.<key>.exclusive_groups`

remove the user from supplementary groups not listed in groups

### `bootstrap.users.<key>.home`

home directory; an absolute path without ':', CR or LF

### `bootstrap.users.<key>.shell`

login shell; an absolute path without ':', CR or LF

### `bootstrap.users.<key>.comment`

comment (GECOS) field, without ':', CR or LF

### `bootstrap.users.<key>.system`

create a system user

### `bootstrap.users.<key>.create_home`

create the home directory

### `bootstrap.users.<key>.move_home`

move the existing home directory to home

### `bootstrap.users.<key>.remove_home`

remove the home directory when removing the user

## `daemon_groups`

Experimental named groups of project daemons. Members are daemon names or other group names declared in the same project.

### `daemon_groups.<name>`

Daemon or group names in this project.

### `daemon_groups.<name>.daemons`

Daemon or group names in this project.

## `daemon_providers`

Experimental shared servers owned by the user. Only allowed in global configuration.

### `daemon_providers.<name>`

A PostgreSQL, CockroachDB or NATS server that every checkout shares, with a separate database or NATS account for each.

### `daemon_providers.<name>.preset`

Service preset that runs the server.

### `daemon_providers.<name>.version`

Version request for the preset's tool.

### `daemon_providers.<name>.port`

A fixed port, or "auto" to derive one from the project root so the primary checkout keeps the base port and each linked git worktree gets its own.

### `daemon_providers.<name>.port.auto`

Enable per-worktree port allocation.

### `daemon_providers.<name>.port.base`

Port used by the primary checkout. Defaults to the preset's port; required for custom daemons.

### `daemon_providers.<name>.port.stride`

Distance between consecutive worktree slots. Defaults to 1.

### `daemon_providers.<name>.ports`

Overrides for the preset's additional named ports.

### `daemon_providers.<name>.ports.<key>`

Port for this named port of the preset: a fixed port, or an allocation that gives each linked git worktree its own.

### `daemon_providers.<name>.ports.<key>.auto`

Enable per-worktree allocation of this named port.

### `daemon_providers.<name>.ports.<key>.base`

Port used by the primary checkout. Defaults to the preset's default for this port; independent of the daemon's own `port`.

### `daemon_providers.<name>.ports.<key>.stride`

Distance between consecutive worktree slots. Defaults to 1.

### `daemon_providers.<name>.options`

Preset options, such as `database`.

### `daemon_providers.<name>.data_dir`

Persistent server data directory; a relative path resolves inside the provider's state directory. Defaults to $MISE\_STATE\_DIR/daemon-providers/\<name>/data.

### `daemon_providers.<name>.tool`

Tool to install the server from, in place of the preset's default.

## `daemons`

Experimental project daemons: custom pitchfork commands or managed presets.

### `daemons.<name>`

A daemon: a version request for the preset of the same name, or a table describing a custom command, task, preset, another project's daemon, or a shared provider.

### `daemons.<name>.run`

Shell command for the daemon's long-running process.; or Shell command that replaces the preset's command.

### `daemons.<name>.init`

idempotent setup commands run before the daemon's long-running process

### `daemons.<name>.mise`

false runs the process and its setup without the project's tool environment.; or false runs setup commands and readiness checks without the project's tool environment.

### `daemons.<name>.port`

A fixed port, or "auto" to derive one from the project root so the primary checkout keeps the base port and each linked git worktree gets its own.

### `daemons.<name>.port.auto`

Enable per-worktree port allocation.

### `daemons.<name>.port.base`

Port used by the primary checkout. Defaults to the preset's port; required for custom daemons.

### `daemons.<name>.port.stride`

Distance between consecutive worktree slots. Defaults to 1.

### `daemons.<name>.proxy`

Hostname label the reverse proxy routes to this daemon, false to opt out, or true to use the daemon's name. Only a daemon with a port is routed. The hostname is \<proxy>.\<worktree>.\<project>.\<tld> in a linked git worktree and \<proxy>.\<project>.\<tld> in the primary checkout.

### `daemons.<name>.proxy_tls`

Whether the proxy terminates TLS for this daemon or passes the connection through to it.

### `daemons.<name>.proxy_idle_timeout`

Stop this daemon after this long without proxy traffic when the proxy started it, for example "30m". false exempts it from idle shutdown even when pitchfork's proxy.idle\_timeout is set; "0" does the same. Idle shutdown is off unless this or pitchfork's proxy.idle\_timeout is set, and a daemon started with `mise daemons start` is never stopped for inactivity. Requires pitchfork 2.27.0.

### `daemons.<name>.task`

mise task run as this daemon's long-running process

### `daemons.<name>.args`

arguments passed to the task after `--`

### `daemons.<name>.data_dir`

Persistent preset data directory, relative to the declaring project root or absolute. Defaults to mise state storage.; or Host directory mounted at data\_mount, relative to the declaring project root or absolute. Defaults to mise state storage.

### `daemons.<name>.preset`

Service preset that supplies the daemon's command, port and data directory.

### `daemons.<name>.version`

Version request for the preset's tool.

### `daemons.<name>.ports`

Overrides for the preset's additional named ports.; or More published ports by name, each exported as \<NAME>\_\<PORT>\_PORT.

### `daemons.<name>.ports.<key>`

Port for this named port of the preset: a fixed port, or an allocation that gives each linked git worktree its own.

### `daemons.<name>.ports.<key>.auto`

Enable per-worktree allocation of this named port.; or Enable per-worktree port allocation, instead of a fixed host port.

### `daemons.<name>.ports.<key>.base`

Port used by the primary checkout. Defaults to the preset's default for this port; independent of the daemon's own `port`.; or Host port used by the primary checkout. Defaults to the container port.

### `daemons.<name>.ports.<key>.stride`

Distance between consecutive worktree slots. Defaults to 1.

### `daemons.<name>.ports.<key>.container`

Port the container listens on.

### `daemons.<name>.ports.<key>.host`

Fixed host port. Defaults to the container port.

### `daemons.<name>.options`

Preset options, such as `database`.

### `daemons.<name>.image`

Container image the daemon runs with Docker or Podman, for example `postgres:18`. mise runs it in the foreground, so its output is the daemon's log, and stops it when the daemon stops.

### `daemons.<name>.runtime`

Docker or Podman. `auto` uses Docker when it is on PATH, otherwise Podman. The MISE\_CONTAINER\_RUNTIME environment variable overrides this with any docker-compatible CLI.

### `daemons.<name>.command`

The command and arguments the container runs in place of the image's default: shell text, or an argument array.

### `daemons.<name>.container_port`

Port the container listens on for `port`. Defaults to the port's `base`, or to the port itself when it is fixed.

### `daemons.<name>.volumes`

Volumes as `source:target[:options]`. A source starting with `.`, `/` or `~` is a path, relative to the project root, and is created when missing; anything else is a named volume.

### `daemons.<name>.data_mount`

Path inside the container where a host data directory is mounted. The directory is data\_dir, or per-checkout mise state storage by default.

### `daemons.<name>.runtime_args`

Extra arguments for `<runtime> run`, such as `--shm-size=1g`, placed before the image.

### `daemons.<name>.project`

Directory of a project whose mise configuration declares the daemon. A relative path resolves against this project root.

### `daemons.<name>.name`

Daemon name inside the referenced project, when it differs from this key.

### `daemons.<name>.provider`

Name of a globally defined shared server provider.

### `daemons.<name>.resource`

Database or NATS account shared by consumers choosing this name. Omit for a checkout-specific resource.

## `daemons_settings`

Experimental project-wide daemon options.

### `daemons_settings.namespace`

Fixed pitchfork namespace for this project, replacing the hashed default so other projects can refer to its daemons by qualified ID.

### `daemons_settings.namespace_per_worktree`

Append a worktree-specific suffix to an explicit namespace when the project root is a linked git worktree.

## `deps`

configure deps providers

### `deps.disable`

Disable specific deps providers

### `deps.npm`

Prepare provider configuration

### `deps.npm.auto`

Auto-run before `mise x` and `mise run`

### `deps.npm.run`

Command to run when stale

### `deps.npm.sources`

Files/patterns to check for changes

### `deps.npm.outputs`

Files/directories that should be newer than sources

### `deps.npm.env`

Environment variables to set

### `deps.npm.env.<NAME>`

value of this environment variable

### `deps.npm.dir`

Working directory for the command

### `deps.npm.description`

Description shown in output

### `deps.npm.depends`

Other deps providers that must complete before this one runs

### `deps.npm.timeout`

Timeout for the run command (e.g., "30s", "5m", "1h")

### `deps.yarn`

Prepare provider configuration

### `deps.pnpm`

Prepare provider configuration

### `deps.bun`

Prepare provider configuration

### `deps.aube`

Prepare provider configuration

### `deps.deno`

Prepare provider configuration

### `deps.go`

Prepare provider configuration

### `deps.pip`

Prepare provider configuration

### `deps.poetry`

Prepare provider configuration

### `deps.uv`

Prepare provider configuration

### `deps.bundler`

Prepare provider configuration

### `deps.composer`

Prepare provider configuration

### `deps.dart`

Prepare provider configuration

### `deps.flutter`

Prepare provider configuration

### `deps.git-submodule`

Prepare provider configuration

### `deps.<key>`

Prepare provider configuration

## `doctor`

Project diagnostic checks, run explicitly with mise doctor project.

### `doctor.checks`

Checks run by mise doctor project, keyed by check name.

### `doctor.checks.<key>`

A check: a command that passes when it exits zero.

### `doctor.checks.<key>.run`

Diagnostic command. Exit zero to pass; other exit codes fail.

### `doctor.checks.<key>.description`

Human-readable requirement checked by the command.

### `doctor.checks.<key>.hint`

Remediation guidance displayed on failure; never executed.

### `doctor.checks.<key>.timeout`

Positive command timeout, such as 5s. Defaults to 10s.

### `doctor.checks.<key>.dir`

Working directory. Relative paths resolve from the declaring config's root (the invocation directory for global and system config); a leading \~/ and absolute paths are used as given.

### `doctor.checks.<key>.shell`

Shell command, including the command flag, as in tasks.

### `doctor.checks.<key>.os`

OS or OS/arch selectors; omit to run everywhere.

### `doctor.checks.<key>.depends`

Names of checks that must pass first. If one does not pass, this check is reported as blocked and its command does not run.

## `dotenv`

dotenv file(s) to load. Deprecated; use env.\_.file instead. This will be removed in mise 2027.4.0.

## `dotfile_groups`

named directory trees of dotfiles, applied with `mise dotfiles apply` or `mise bootstrap` while \[bootstrap] dotfile\_groups selects them (or is unset)

### `dotfile_groups.<name>`

a directory tree of dotfiles deployed into a target directory

### `dotfile_groups.<name>.root`

the directory tree the group deploys; a relative path resolves against dotfiles.root

### `dotfile_groups.<name>.target`

target directory, absolute or starting with \~/

### `dotfile_groups.<name>.depends`

dotfile groups (by name) and whole-file entries (by target path) mise writes before it deploys this group and its entries

### `dotfile_groups.<name>.mode`

how the tree is deployed

### `dotfile_groups.<name>.exclude`

glob patterns of source paths to skip, as for \[dotfiles] exclude

### `dotfile_groups.<name>.dot_prefix`

deploy each source path component named "dot-\<name>" as ".\<name>", like GNU Stow's --dotfiles

### `dotfile_groups.<name>.manifest`

source manifest used to select managed files from the tree

### `dotfile_groups.<name>.relative`

link by a path relative to the link's directory; overrides the dotfiles.relative\_symlinks setting (ignored on Windows)

### `dotfile_groups.<name>.entries`

entries for parts of the tree, written like \[dotfiles] entries and applied only while the group is selected. A whole-file entry is keyed by its target path inside the group's target and cut out of the walk; a block, line, or merge edit is keyed by the path of the file it edits, inside the group's target, then "/" and an id. A missing source is found under the root at the entry's path inside the target, and a relative source starts at the root

### `dotfile_groups.<name>.entries.<key>`

an entry for a path inside the group's target: a whole-file entry's source path, or a table as in \[dotfiles] for a whole-file entry or a block, line, or merge edit

### `dotfile_groups.<name>.entries.<key>.encrypt`

encrypt tracked file contents before storing them in Git; the live file remains plaintext

### `dotfile_groups.<name>.entries.<key>.allow_plaintext`

allow this directly tracked credential-named file to be saved in plaintext history and any connected origin

### `dotfile_groups.<name>.entries.<key>.source`

source file or directory

### `dotfile_groups.<name>.entries.<key>.content`

literal inline file content; alternative to source

### `dotfile_groups.<name>.entries.<key>.mode`

how to apply a whole-file source to the target; omitted uses the group's mode. "absent" removes a file or symlink at the target and takes no source. "track" saves the file's history where it is, and "track-local" saves it in this machine's local-only history, which is never shared

### `dotfile_groups.<name>.entries.<key>.exclude`

glob patterns to skip when the mode walks a directory (symlink-each, copy with a directory source, or a tracked directory, where they are relative to the tracked path); a pattern without "/" matches any path component, one with "/" is anchored to the root

### `dotfile_groups.<name>.entries.<key>.include`

history only (mode = "track"): the only paths of the tracked directory that are captured, relative to it and matched like "exclude"; without it the whole tree is captured with the builtin credential filtering, and an explicit "exclude" still wins. A selected file is captured even when it looks like a credential store, in plaintext unless the entry sets encrypt

### `dotfile_groups.<name>.entries.<key>.manifest`

source manifest used to select managed files from a directory

### `dotfile_groups.<name>.entries.<key>.remove_empty`

template mode only: remove the target when the template renders empty or whitespace-only; a target that changed since mise last wrote it needs --force

### `dotfile_groups.<name>.entries.<key>.create_if_missing`

copy and template modes, or inline content: write the target only when nothing exists there yet, leaving an existing file's content and permissions alone; a target that was deleted is created again on the next apply. Not for directory sources

### `dotfile_groups.<name>.entries.<key>.dot_prefix`

symlink-each and directory copy only: deploy each source path component named "dot-\<name>" as ".\<name>", like GNU Stow's --dotfiles; exclude still matches source names

### `dotfile_groups.<name>.entries.<key>.relative`

symlink and symlink-each only: link by a path relative to the link's directory instead of an absolute one; overrides the dotfiles.relative\_symlinks setting (ignored on Windows)

### `dotfile_groups.<name>.entries.<key>.permissions`

octal permissions for the target, such as "0600"; overrides what copy, template, and inline content entries set, and on its own manages only the permissions of an existing target; on a merge entry it sets the mode only when apply creates the target, never on one that exists (Unix only)

### `dotfile_groups.<name>.entries.<key>.autosave`

track only: false saves the path only on `mise dotfiles save <path>`

### `dotfile_groups.<name>.entries.<key>.enabled`

false turns off an entry inherited from another config file

### `dotfile_groups.<name>.entries.<key>.variants`

OS/profile variants: history streams for track mode, or optional destinations for deployment modes

### `dotfile_groups.<name>.entries.<key>.depends`

dotfile groups (by name) and whole-file entries (by target path) mise writes before it renders or reads this entry, in addition to the group's own. Not for edits or track modes

### `dotfile_groups.<name>.entries.<key>.block`

edit entry: block content, maintained between marker comments

### `dotfile_groups.<name>.entries.<key>.template`

template engine to render block content with; currently only "tera"

### `dotfile_groups.<name>.entries.<key>.line`

edit entry: exact line to ensure exists

### `dotfile_groups.<name>.entries.<key>.position`

where to insert a missing line

### `dotfile_groups.<name>.entries.<key>.comment`

comment prefix for edit marker lines; inferred from the file extension when omitted

### `dotfile_groups.<name>.entries.<key>.merge`

edit entry: true sets the keys of source (default: the file's path under the group root) on a JSON, JSONC, TOML, or YAML target and leaves the target's other keys to the application that also writes it; "missing" sets only the keys the target has no value for; the format comes from the target's extension unless "format" names it

### `dotfile_groups.<name>.entries.<key>.format`

merge only: the format of the target and source when the target's extension doesn't name it, such as "jsonc" for VS Code's settings.json

## `dotfiles`

dotfiles applied with `mise dotfiles apply` or `mise bootstrap`, keyed by target path or a logical name when every variant overrides the target; an omitted source for the latter resolves the relative key under dotfiles.root

### `dotfiles.<target>`

a dotfile: the source path of a whole-file entry, or a table describing a whole-file entry or an edit

### `dotfiles.<target>.encrypt`

encrypt tracked file contents before storing them in Git; the live file remains plaintext

### `dotfiles.<target>.allow_plaintext`

allow this directly tracked credential-named file to be saved in plaintext history and any connected origin

### `dotfiles.<target>.source`

source file or directory

### `dotfiles.<target>.content`

literal inline file content; alternative to source

### `dotfiles.<target>.mode`

how to apply a whole-file source to the target; omitted uses dotfiles.default\_mode. "absent" removes a file or symlink at the target and takes no source. "track" saves the file's history where it is, and "track-local" saves it in this machine's local-only history, which is never shared. "fnox" writes the file fnox stores for the target (`fnox set --file`) with mode 0600 and takes no source

### `dotfiles.<target>.exclude`

glob patterns to skip when the mode walks a directory (symlink-each, copy with a directory source, or a tracked directory, where they are relative to the tracked path); a pattern without "/" matches any path component, one with "/" is anchored to the root

### `dotfiles.<target>.include`

history only (mode = "track"): the only paths of the tracked directory that are captured, relative to it and matched like "exclude"; without it the whole tree is captured with the builtin credential filtering, and an explicit "exclude" still wins. A selected file is captured even when it looks like a credential store, in plaintext unless the entry sets encrypt

### `dotfiles.<target>.manifest`

source manifest used to select managed files from a directory

### `dotfiles.<target>.remove_empty`

template mode only: remove the target when the template renders empty or whitespace-only; a target that changed since mise last wrote it needs --force

### `dotfiles.<target>.create_if_missing`

copy and template modes, or inline content: write the target only when nothing exists there yet, leaving an existing file's content and permissions alone; a target that was deleted is created again on the next apply. Not for directory sources

### `dotfiles.<target>.dot_prefix`

symlink-each and directory copy only: deploy each source path component named "dot-\<name>" as ".\<name>", like GNU Stow's --dotfiles; exclude still matches source names

### `dotfiles.<target>.relative`

symlink and symlink-each only: link by a path relative to the link's directory instead of an absolute one; overrides the dotfiles.relative\_symlinks setting (ignored on Windows)

### `dotfiles.<target>.permissions`

octal permissions for the target, such as "0600"; overrides what copy, template, and inline content entries set, and on its own manages only the permissions of an existing target; on a merge entry it sets the mode only when apply creates the target, never on one that exists (Unix only)

### `dotfiles.<target>.autosave`

track only: false saves the path only on `mise dotfiles save <path>`

### `dotfiles.<target>.enabled`

false turns off an entry inherited from another config file

### `dotfiles.<target>.block`

block content, maintained between marker comments

### `dotfiles.<target>.template`

template engine to render block content with; currently only "tera"

### `dotfiles.<target>.line`

exact line to ensure exists

### `dotfiles.<target>.position`

where to insert a missing line

### `dotfiles.<target>.comment`

comment prefix for edit marker lines; inferred from the file extension when omitted

### `dotfiles.<target>.merge`

edit entry: true sets the keys of source (default: the target's path under dotfiles.root) on a JSON, JSONC, TOML, YAML, or INI target and leaves the target's other keys to the application that also writes it; "missing" sets only the keys the target has no value for; the format comes from the target's extension unless "format" names it

### `dotfiles.<target>.format`

merge only: the format of the target and source when the target's extension doesn't name it, such as "jsonc" for VS Code's settings.json or "ini" for an extensionless INI file

### `dotfiles.<target>.variants`

OS/profile variants: history streams for track mode, or optional destinations for deployment modes

### `dotfiles.<target>.variants[].os`

operating system (linux, macos, windows, or unix), optionally with an architecture such as linux/arm64; a list matches any of its values

### `dotfiles.<target>.variants[].profile`

mise environment selected with -E or MISE\_ENV

### `dotfiles.<target>.variants[].default`

true marks the fallback for machines that match no other variant; takes no os or profile

### `dotfiles.<target>.variants[].machine`

track mode only: a separate stream on every machine, never applied on another machine; must be the only variant and cannot be combined with encrypt

### `dotfiles.<target>.variants[].target`

deployment destination override; requires either an explicit source or a safe relative entry key when every variant has a target, and is not supported in track mode

### `dotfiles.<target>.depends`

dotfile groups (by name) and whole-file entries (by target path) mise writes before it renders or reads this entry, so a template, a fnox file or a copy can use files they create. A dependency that fails stops this entry and everything after it; a cycle or a name that is neither is an error. Not for edits or track modes

## `env`

environment variables to set

### `env._`

environment modules

### `env._.file`

files to load variables from (dotenv, json, yaml, or toml)

### `env._.file.path`

file to load, or a list of them; may be a glob pattern

### `env._.file.paths`

files to load; each may be a glob pattern

### `env._.file.tools`

load tools before resolving

### `env._.file.expand`

enable shell-style variable expansion for values loaded by env.\_.file

### `env._.file.find_up`

search the config file's directory and each parent directory for a relative env.\_.file path. An absolute path searches its own directory and each parent for its file name

### `env._.file.redact`

redact the value from logs

### `env._.file.required`

require this environment variable to be defined before mise runs or in a later config file. Can be a boolean or a help string. A variable that carries its own `value` cannot also be `required = true`.

### `env._.path`

directories to add to PATH, ahead of tool paths unless `position = "append"`

### `env._.path.path`

directory to add to PATH, or a list of them

### `env._.path.paths`

directories to add to PATH

### `env._.path.tools`

load tools before resolving

### `env._.path.position`

where env.\_.path puts the directory. prepend (the default) adds it ahead of tool paths. append adds it after tool paths and ahead of the inherited PATH

### `env._.python`

python environment

### `env._.python.venv`

python virtual environment to activate

### `env._.python.venv.create`

create a new virtual environment if one does not exist

### `env._.python.venv.path`

path to python virtual environment to use

### `env._.python.venv.python`

python version to use

### `env._.python.venv.python_create_args`

additional arguments to pass to python when creating a virtual environment

### `env._.python.venv.uv_create_args`

additional arguments to pass to uv when creating a virtual environment

### `env._.source`

bash scripts to run, taking the variables they export

### `env.<NAME>`

an environment variable: its value, or a table with `value`, `default`, `required` or `age`

### `env.<NAME>.value`

environment variable value; false removes the variable

### `env.<NAME>.tools`

load tools before resolving

### `env.<NAME>.redact`

redact the value from logs

### `env.<NAME>.required`

a variable with its own value cannot be required; only `required = false` is accepted; or a variable without a value must be required: `true` or help text on how to set it; or require this environment variable to be defined before mise runs or in a later config file. Can be a boolean or a help string. A variable that carries its own `value` cannot also be `required = true`.

### `env.<NAME>.default`

fallback value to use when the environment variable is unset or empty

### `env.<NAME>.age`

age-encrypted value (simplified format); or \[experimental] age-encrypted value (complex format)

### `env.<NAME>.age.value`

age-encrypted value

### `env.<NAME>.age.format`

compression format for the encrypted value

### `env.<NAME>.age.tools`

load tools before resolving

### `env.<NAME>.age.redact`

redact the value from logs

### `env.<NAME>.age.required`

require this environment variable to be defined before mise runs or in a later config file. Can be a boolean or a help string. A variable that carries its own `value` cannot also be `required = true`.

## `env_file`

dotenv file(s) to load. Deprecated; use env.\_.file instead. This will be removed in mise 2027.4.0.

## `env_path`

PATH entries to add. Deprecated; use env.\_.path instead. This will be removed in mise 2027.4.0.

## `experimental_monorepo_root`

deprecated alias of monorepo\_root

## `history`

dotfiles history: what is never captured, reload commands, and the setup repository (<https://mise.jdx.dev/history.html>)

### `history.encryption`

encryption for dotfiles tracked with encrypt = true

### `history.encryption.recipients`

shared age, SSH, or age-plugin public recipients for all encrypted dotfiles

### `history.exclude`

globs never captured by dotfiles history; a `!glob` entry re-includes a path an earlier glob excluded

### `history.git_email`

email used for history commit authors and committers; {hostname} expands to this machine's hostname

### `history.machine`

this machine's name in per-machine streams (variants = \[{ machine = true }]) instead of the name generated with the history store

### `history.reload`

commands run once after a rollback, undo, pull, or dotfiles apply writes a path matching the glob (trusted global or system config only)

### `history.reload.<key>`

command to run once when a rollback, undo, pull, or dotfiles apply writes a path matching this glob

### `history.origin`

the setup repository this machine publishes to and fetches from; machine-local, written by `mise bootstrap dotfiles origin set`

### `history.origin.url`

the repository url

### `history.origin.branch`

the setup branch

## `hooks`

hooks to run on events like cd, enter, leave

### `hooks.enter`

hooks to run when an activated shell moves into the project

### `hooks.enter.run`

inline hook command to run

### `hooks.enter.run_windows`

inline hook command to run on Windows

### `hooks.enter.shell`

inline shell command to use for run hooks, such as "bash -c"; or active shell name for current-shell hooks, such as "bash"

### `hooks.enter.script`

script or scripts to evaluate in the active shell for enter/leave/cd hooks; or deprecated alias for run when an enter/leave/cd hook has no shell

### `hooks.enter.scripts`

scripts to evaluate in the active shell for enter/leave/cd hooks; or deprecated alias for run when an enter/leave/cd hook has no shell

### `hooks.enter.task`

mise task to run

### `hooks.leave`

hooks to run when an activated shell moves out of the project

### `hooks.cd`

hooks to run when an activated shell changes to a directory inside the project

### `hooks.preinstall`

hooks to run before mise installs tools

### `hooks.preinstall.run`

inline hook command to run

### `hooks.preinstall.run_windows`

inline hook command to run on Windows

### `hooks.preinstall.shell`

inline shell command to use for run hooks, such as "bash -c"; or ignored for preinstall/postinstall hooks that use script; use a run hook to select an inline shell; or ignored for preinstall/postinstall hooks that use scripts; use a run hook to select an inline shell

### `hooks.preinstall.script`

deprecated alias for run in preinstall/postinstall hooks; executes in a subprocess

### `hooks.preinstall.scripts`

deprecated alias for run in preinstall/postinstall hooks; joins the scripts and executes them in a subprocess

### `hooks.preinstall.task`

mise task to run

### `hooks.postinstall`

hooks to run after mise installs tools

## `include`

config files merged into this one, ranking just below it: a local path (relative to this file), git::\<url>//\<path>.toml?ref=\<ref> or oci::\<registry>/\<repo>\[:tag|@sha256:\<digest>] (the artifact holds a mise.toml). Paranoid mode requires a commit sha or digest for remote includes and rejects local ones; otherwise a branch or tag is refreshed after fetch\_remote\_versions\_cache

## `min_version`

minimum version of mise required to use this config

### `min_version.hard`

error if current mise is lower than this version

### `min_version.soft`

warn if current mise is lower than this version

## `monorepo`

configuration for monorepo task discovery

### `monorepo.config_roots`

Explicit list of config root paths for monorepo task discovery. Supports single-level glob patterns (\*). When set, skips filesystem walking for better performance.

### `monorepo.path_aliases`

Short names for configured monorepo task roots. Keys are single path segments; values are paths in config\_roots.

### `monorepo.path_aliases.<key>`

Config root this alias names, as listed in config\_roots

### `monorepo.lockfile`

Use a single lockfile at the monorepo root for descendant config roots. true opts in now, false keeps lockfiles next to subproject configs, and unset keeps the current behavior until the scheduled default flip.

### `monorepo.task_defaults`

Task defaults applied by task name across inferred and explicit workspace projects

### `monorepo.task_defaults.<key>`

task template that can be extended by tasks

### `monorepo.projects`

Explicit additions, removals, and overrides applied to provider-inferred workspace projects

### `monorepo.projects.<key>`

Changes to the workspace project with this ID, or a new project when the ID is new

### `monorepo.projects.<key>.remove`

Remove this project and every dependency edge connected to it

### `monorepo.projects.<key>.root`

Add the project at this workspace-relative root or replace its inferred root

### `monorepo.projects.<key>.metadata`

Replace provider-inferred project metadata

### `monorepo.projects.<key>.metadata.<key>`

### `monorepo.projects.<key>.depends`

Replace the complete provider-inferred dependency set

### `monorepo.projects.<key>.depends_add`

Add dependency edges after an optional depends replacement

### `monorepo.projects.<key>.depends_remove`

Remove dependency edges after an optional depends replacement

## `monorepo_root`

marks this config as a monorepo root for task path syntax

## `oci`

configuration for `mise oci build`

### `oci.from`

Base image reference (overrides the oci.default\_from setting)

### `oci.tag`

Default tag applied to the built image

### `oci.workdir`

Working directory baked into the image config

### `oci.entrypoint`

Entrypoint baked into the image config

### `oci.cmd`

Cmd baked into the image config

### `oci.user`

User baked into the image config

### `oci.user_id`

Numeric UID assigned to tar layer entries

### `oci.group_id`

Numeric GID assigned to tar layer entries. Defaults to user\_id when unset

### `oci.mount_point`

Override where mise installs go in the image (default /mise)

### `oci.copy`

Host files or directories copied into the image as independent layers

### `oci.copy[].host`

host path to copy, relative to the config file when not absolute

### `oci.copy[].image`

absolute non-root image path without `.` or `..` components

### `oci.env`

Extra env vars baked into the image config

### `oci.env.<NAME>`

value of this environment variable

### `oci.labels`

Labels baked into the image config

### `oci.labels.<key>`

value of this image label

## `plugins`

plugins to use

### `plugins.<name>`

url to plugin repository

## `redactions`

env or vars keys to redact from logs

## `secrets`

Sources for mise secrets: values resolved only when mise starts a task or `mise x` command that was granted them. Project config only.

### `secrets.fnox`

Use fnox (fnox.toml discovered from this config's directory). <https://mise.jdx.dev/environments/secrets/fnox.html>

### `secrets.fnox.profile`

fnox profile (fnox -P); defaults to FNOX\_PROFILE, then "default"

## `settings`

mise settings

## `shell_alias`

shell aliases

### `shell_alias.<name>`

command to run

## `task_config`

configuration for task execution and management

### `task_config.cascade`

cascade task configuration to descendant config roots

### `task_config.dir`

default directory to run tasks in defined in this file

### `task_config.shell`

default shell to run tasks with in this config scope

### `task_config.cache`

default experimental local artifact cache configuration for eligible tasks with sources and explicit outputs

### `task_config.cache.enabled`

store and restore eligible task outputs from the local mise cache

### `task_config.cache.audit`

report project files read or written outside declared cache sources and outputs

### `task_config.cache.env`

ambient environment variable names whose values affect eligible task cache keys

### `task_config.cache.command_inputs`

commands whose stdout and stderr affect eligible task cache keys

### `task_config.rust_cache`

deprecated no-op; use mbx for Rust action caching instead: <https://mr-boxington.jdx.dev/getting-started>

### `task_config.rust_cache.enabled`

deprecated no-op

### `task_config.rust_cache.verify`

deprecated no-op

### `task_config.global_env`

experimental ambient environment variable names added to every enabled task cache key in this config scope

### `task_config.global_pass_through_env`

experimental ambient environment variable names preserved under environment denial without affecting task cache keys

### `task_config.global_inputs`

experimental config-root-relative source patterns added to every task in this config scope; entries may reference input groups with @group:\<name>

### `task_config.input_groups`

experimental reusable config-root-relative source groups referenced with @group:\<name>

### `task_config.input_groups.<key>`

source patterns in this group, relative to the config root; may reference other groups with @group:\<name>

### `task_config.includes`

files/directories to include searching for tasks. Can be local paths, git repository URLs using git:: prefix (e.g., git:: <https://github.com/org/repo.git//path?ref=branch>), or OCI artifacts using oci:: prefix (e.g., oci::ghcr.io/org/tasks:1.0.0)

### `task_config.excludes`

config-root-relative paths or glob patterns to exclude from file-task discovery

## `task_templates`

task templates that can be extended by tasks via extends

### `task_templates.<name>`

task template that can be extended by tasks

### `task_templates.<name>.alias`

alias for this task

### `task_templates.<name>.confirm`

prompt to answer before this task's own command runs, after its dependencies

### `task_templates.<name>.depends`

tasks that must finish before this task runs

### `task_templates.<name>.daemons`

project daemons that must be running and ready before this task runs

### `task_templates.<name>.depends_post`

tasks to run after this task finishes

### `task_templates.<name>.wait_for`

tasks to wait for if they are already scheduled, without adding them to the run

### `task_templates.<name>.description`

description of task

### `task_templates.<name>.tags`

### `task_templates.<name>.dir`

directory to run script in, default is the project's base directory

### `task_templates.<name>.env`

environment variables

### `task_templates.<name>.vars`

variables to set

### `task_templates.<name>.tools`

tools to install/activate before running this task

### `task_templates.<name>.outputs`

files or directories this task creates; compared with `sources` to skip the task when it is up to date

### `task_templates.<name>.outputs.auto`

automatically touch an internal tracked file instead of specifying outputs; `false` is the same as leaving `outputs` unset

### `task_templates.<name>.cache`

experimental local artifact cache for declared task outputs

### `task_templates.<name>.rust_cache`

deprecated no-op; use mbx for Rust action caching instead: <https://mr-boxington.jdx.dev/getting-started>

### `task_templates.<name>.rust_cache.enabled`

deprecated no-op

### `task_templates.<name>.rust_cache.verify`

deprecated no-op

### `task_templates.<name>.output`

output style for this task (prefix, interleave, keep-order, replacing, summary, timed, quiet, silent). Independent of the quiet/silent verbosity fields.

### `task_templates.<name>.silent`

suppress all output for this task

### `task_templates.<name>.run`

script to run

### `task_templates.<name>.run_windows`

script to run

### `task_templates.<name>.file`

Execute an external script

### `task_templates.<name>.sources`

files this task reads; mise skips the task when none changed since its outputs were written

### `task_templates.<name>.watch`

options used by `mise watch` for this task

### `task_templates.<name>.watch.no_vcs_ignore`

do not load VCS ignore files such as `.gitignore` when watching this task

### `task_templates.<name>.shell`

specify a shell command to run the script with

### `task_templates.<name>.usage`

Specify usage (<https://usage.jdx.dev/>) specs for the task

### `task_templates.<name>.timeout`

timeout for this task

### `task_templates.<name>.lock`

resources this task holds while its commands run, so runs that need the same one wait their turn

### `task_templates.<name>.skip_if`

shell command checked before the task runs; if it exits 0, the task is skipped and counts as successful

### `task_templates.<name>.deny_all`

block reads, writes, network, and env vars

### `task_templates.<name>.deny_read`

block filesystem reads

### `task_templates.<name>.deny_write`

block all filesystem writes

### `task_templates.<name>.deny_net`

block all network access

### `task_templates.<name>.deny_env`

block env var inheritance

### `task_templates.<name>.allow_read`

allow reads from specific paths

### `task_templates.<name>.allow_write`

allow writes to specific paths

### `task_templates.<name>.allow_net`

allow network to specific hosts; not supported yet, so Linux and macOS reject it

### `task_templates.<name>.allow_env`

allow specific env vars through

### `task_templates.<name>.pass_through_env`

experimental ambient env vars preserved under environment denial without affecting the task cache key

## `tasks`

task runner tasks

### `tasks.<name>`

a task: a script, a list of scripts and task references, or a table

### `tasks.<name>.alias`

alias for this task

### `tasks.<name>.confirm`

prompt to answer before this task's own command runs, after its dependencies

### `tasks.<name>.confirm.message`

confirmation message before running this task

### `tasks.<name>.confirm.default`

default value for confirmation (yes/no/true/false)

### `tasks.<name>.confirm.yes`

custom label for the affirmative answer

### `tasks.<name>.confirm.no`

custom label for the negative answer

### `tasks.<name>.depends`

tasks that must finish before this task runs

### `tasks.<name>.depends.task`

task name, alias, or pattern

### `tasks.<name>.depends.args`

arguments to pass to the task

### `tasks.<name>.depends.env`

environment variables to set for this dependency only

### `tasks.<name>.depends.env.<NAME>`

value of this environment variable

### `tasks.<name>.depends.optional`

do not fail when the task name or pattern has no matches

### `tasks.<name>.daemons`

project daemons that must be running and ready before this task runs

### `tasks.<name>.depends_post`

tasks to run after this task finishes

### `tasks.<name>.wait_for`

tasks to wait for if they are already scheduled, without adding them to the run

### `tasks.<name>.description`

description of task

### `tasks.<name>.tags`

### `tasks.<name>.dir`

directory to run script in, default is the project's base directory

### `tasks.<name>.env`

environment variables

### `tasks.<name>.vars`

variables to set

### `tasks.<name>.tools`

tools to install/activate before running this task

### `tasks.<name>.tools.<tool>`

a version of the tool, or a table with its version and options

### `tasks.<name>.tools.<github:…>`

a tool installed with the github backend

### `tasks.<name>.tools.<gitlab:…>`

a tool installed with the gitlab backend

### `tasks.<name>.tools.<forgejo:…>`

a tool installed with the forgejo backend

### `tasks.<name>.tools.<ubi:…>`

a tool installed with the ubi backend

### `tasks.<name>.tools.<http:…>`

a tool installed with the http backend

### `tasks.<name>.tools.<s3:…>`

a tool installed with the s3 backend

### `tasks.<name>.tools.<aqua:…>`

a tool installed with the aqua backend

### `tasks.<name>.tools.<cargo:…>`

a tool installed with the cargo backend

### `tasks.<name>.tools.<npm:…>`

a tool installed with the npm backend

### `tasks.<name>.tools.<pypi:…>`

a tool installed with the pypi backend

### `tasks.<name>.tools.<pipx:…>`

a tool installed with the pypi backend

### `tasks.<name>.tools.<gem:…>`

a tool installed with the gem backend

### `tasks.<name>.tools.<gem:…>.source`

gem registry http(s) url used to resolve and install this gem

### `tasks.<name>.tools.<go:…>`

a tool installed with the go backend

### `tasks.<name>.tools.<go:…>.tags`

build tags passed to go install -tags

### `tasks.<name>.tools.<conda:…>`

a tool installed with the conda backend

### `tasks.<name>.tools.<conda:…>.channel`

conda channel to resolve the package from (default conda-forge)

### `tasks.<name>.tools.<spm:…>`

a tool installed with the spm backend

### `tasks.<name>.tools.<packslip:…>`

a tool installed with the packslip backend

### `tasks.<name>.tools.<spinel:…>`

a tool installed with the spinel backend

### `tasks.<name>.tools.python`

the core python tool

### `tasks.<name>.tools.python.patch_sysconfig`

patch sysconfig paths of precompiled python after install (default true, unix only)

### `tasks.<name>.tools.python.virtualenv`

virtualenv path to activate; use env.\_.python.venv instead

### `tasks.<name>.tools.java`

the core java tool

### `tasks.<name>.tools.java.release_type`

jdk metadata channel: ga (default) or ea for early access builds

### `tasks.<name>.tools.rust`

the core rust tool

### `tasks.<name>.tools.dotnet`

the core dotnet tool

### `tasks.<name>.tools.dotnet.runtime`

install a shared runtime instead of the sdk; the version is the runtime version

### `tasks.<name>.tools.dotnet-core`

the core dotnet tool

### `tasks.<name>.tools.dotnet-core.runtime`

install a shared runtime instead of the sdk; the version is the runtime version

### `tasks.<name>.hide`

do not display this task

### `tasks.<name>.outputs`

files or directories this task creates; compared with `sources` to skip the task when it is up to date

### `tasks.<name>.outputs.auto`

automatically touch an internal tracked file instead of specifying outputs; `false` is the same as leaving `outputs` unset

### `tasks.<name>.cache`

experimental local artifact cache for declared task outputs

### `tasks.<name>.cache.enabled`

store and restore task outputs from the local mise cache

### `tasks.<name>.cache.audit`

report project files read or written outside declared cache sources and outputs

### `tasks.<name>.cache.env`

ambient environment variable names whose values affect the cache key

### `tasks.<name>.cache.command_inputs`

commands whose stdout and stderr affect the cache key

### `tasks.<name>.rust_cache`

deprecated no-op; use mbx for Rust action caching instead: <https://mr-boxington.jdx.dev/getting-started>

### `tasks.<name>.rust_cache.enabled`

deprecated no-op

### `tasks.<name>.rust_cache.verify`

deprecated no-op

### `tasks.<name>.output`

output style for this task (prefix, interleave, keep-order, replacing, summary, timed, quiet, silent). Independent of the quiet/silent verbosity fields.

### `tasks.<name>.quiet`

do not display mise information for this task

### `tasks.<name>.silent`

suppress all output for this task

### `tasks.<name>.interactive`

mark task as interactive, acquiring exclusive terminal access while allowing other non-interactive tasks to run in parallel

### `tasks.<name>.raw`

directly connect task to stdin/stdout/stderr

### `tasks.<name>.raw_args`

skip mise's argument parsing for this task — all arguments (including --help/-h) are forwarded verbatim to the underlying command. Use for tasks that proxy a tool with its own argument parser.

### `tasks.<name>.run`

script to run

### `tasks.<name>.run.task`

single task name to run

### `tasks.<name>.run.args`

arguments to pass to the task

### `tasks.<name>.run.env`

environment variables to set for the task

### `tasks.<name>.run.env.<NAME>`

value of this environment variable

### `tasks.<name>.run.tasks`

parallel task group to run

### `tasks.<name>.run_windows`

script to run

### `tasks.<name>.file`

Execute an external script

### `tasks.<name>.sources`

files this task reads; mise skips the task when none changed since its outputs were written

### `tasks.<name>.watch`

options used by `mise watch` for this task

### `tasks.<name>.watch.no_vcs_ignore`

do not load VCS ignore files such as `.gitignore` when watching this task

### `tasks.<name>.shell`

specify a shell command to run the script with

### `tasks.<name>.usage`

Specify usage (<https://usage.jdx.dev/>) specs for the task

### `tasks.<name>.timeout`

timeout for this task

### `tasks.<name>.lock`

resources this task holds while its commands run, so runs that need the same one wait their turn

### `tasks.<name>.skip_if`

shell command checked before the task runs; if it exits 0, the task is skipped and counts as successful

### `tasks.<name>.deny_all`

block reads, writes, network, and env vars

### `tasks.<name>.deny_read`

block filesystem reads

### `tasks.<name>.deny_write`

block all filesystem writes

### `tasks.<name>.deny_net`

block all network access

### `tasks.<name>.deny_env`

block env var inheritance

### `tasks.<name>.allow_read`

allow reads from specific paths

### `tasks.<name>.allow_write`

allow writes to specific paths

### `tasks.<name>.allow_net`

allow network to specific hosts; not supported yet, so Linux and macOS reject it

### `tasks.<name>.allow_env`

allow specific env vars through

### `tasks.<name>.pass_through_env`

experimental ambient env vars preserved under environment denial without affecting the task cache key

### `tasks.<name>.secrets`

Secret keys this task receives when it starts, e.g. \["DEPLOY\_KEY"]. Dependencies do not inherit them. See `mise secrets ls`.

### `tasks.<name>.extends`

name of the task template to extend

## `tool_alias`

Tool version aliases

### `tool_alias.<tool>`

the backend this tool name points to, or a table with its backend and version aliases

### `tool_alias.<tool>.backend`

where the alias goes

### `tool_alias.<tool>.versions`

tool version aliases

### `tool_alias.<tool>.versions.<key>`

version alias points to

### `tool_alias.<tool>.<key>`

version alias points to

## `tool_config`

policy for tools declared by this config root

### `tool_config.locked`

require tools declared by this config root to resolve and install from their lockfiles

## `tools`

dev tools to use

### `tools.<tool>`

a version of the tool, or a table with its version and options

### `tools.<tool>.version`

version of the tool to install

### `tools.<tool>.path`

path to a custom compiled version

### `tools.<tool>.prefix`

version prefix to resolve

### `tools.<tool>.ref`

version control reference to compile

### `tools.<tool>.os`

operating system filters to install on

### `tools.<tool>.minimum_release_age`

minimum time since a version's release before mise selects it, as a duration such as "7d" or an absolute date

### `tools.<tool>.install_before`

deprecated alias for minimum\_release\_age

### `tools.<tool>.version_order`

ordering applied to remote versions: "source" keeps the backend's order, "semver" sorts semantic versions

### `tools.<tool>.prerelease`

include pre-release versions when listing versions and resolving latest

### `tools.<tool>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64

### `tools.<tool>.platforms.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<tool>.platform`

per-platform option overrides (alias of platforms)

### `tools.<tool>.platform.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<tool>.auto_update`

update this tool before it runs when its check is due; true, or a check interval such as "12h". Only honored in global config

### `tools.<tool>.install_env`

environment variables during tool installation

### `tools.<tool>.install_env.<key>`

environment variable value; false removes the variable

### `tools.<tool>.depends`

tools that must be installed before this tool

### `tools.<tool>.lazy`

defer installation until one of the tool's bootstrap shims is invoked

### `tools.<tool>.lazy_bins`

bootstrap shim names for lazy tools without registry bin metadata

### `tools.<tool>.postinstall`

command to run after tool installation, or on every install when requested

### `tools.<tool>.postinstall.run`

command to run after the tool installs

### `tools.<tool>.postinstall.when`

install runs the command only when mise installs the tool; always runs it on every mise install and mise use

### `tools.<github:…>`

a tool installed with the github backend

### `tools.<github:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<github:…>.platforms.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<github:…>.platforms.<key>.asset_pattern`

glob (\* and ?) matched against release asset names, tera templated; replaces autodetection

### `tools.<github:…>.platforms.<key>.additional_asset_patterns`

extra archives from the same release extracted over the primary asset, in order

### `tools.<github:…>.platforms.<key>.no_app`

penalize macos .app bundle assets during autodetection

### `tools.<github:…>.platforms.<key>.filter_bins`

only expose these binaries on PATH

### `tools.<github:…>.platforms.<key>.checksum`

expected artifact digest as algo:hash (sha256, sha512, sha1, md5, blake3)

### `tools.<github:…>.platforms.<key>.size`

expected artifact size in bytes

### `tools.<github:…>.platforms.<key>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<github:…>.platforms.<key>.bin`

executable name (or safe relative path) to install the downloaded binary as

### `tools.<github:…>.platforms.<key>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<github:…>.platforms.<key>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<github:…>.platforms.<key>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<github:…>.platforms.<key>.format`

force the archive format instead of detecting it from the file name

### `tools.<github:…>.platforms.<key>.url`

direct download url for this platform, tera templated; skips release asset selection

### `tools.<github:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<github:…>.version_prefix`

release tag prefix stripped from versions and re-added when resolving tags

### `tools.<github:…>.version_regex`

regex a release tag must match to be listed; versions are still derived from the tag as usual

### `tools.<github:…>.asset_pattern`

glob (\* and ?) matched against release asset names, tera templated; replaces autodetection

### `tools.<github:…>.additional_asset_patterns`

extra archives from the same release extracted over the primary asset, in order

### `tools.<github:…>.matching`

substring an asset name must contain, applied before platform autodetection

### `tools.<github:…>.matching_regex`

regex an asset name must match, applied before platform autodetection

### `tools.<github:…>.no_app`

penalize macos .app bundle assets during autodetection

### `tools.<github:…>.filter_bins`

only expose these binaries on PATH

### `tools.<github:…>.format`

force the archive format instead of detecting it from the file name

### `tools.<github:…>.checksum`

expected artifact digest as algo:hash (sha256, sha512, sha1, md5, blake3)

### `tools.<github:…>.size`

expected artifact size in bytes

### `tools.<github:…>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<github:…>.bin`

executable name (or safe relative path) to install the downloaded binary as

### `tools.<github:…>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<github:…>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<github:…>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<github:…>.api_url`

api base url, for github enterprise or self-hosted instances

### `tools.<github:…>.github_attestations`

verify github artifact attestations when available

### `tools.<github:…>.slsa_signer_identity`

expected fulcio certificate identity for slsa provenance, tera templated

### `tools.<github:…>.slsa_signer_issuer`

expected oidc issuer for slsa provenance; requires slsa\_signer\_identity

### `tools.<gitlab:…>`

a tool installed with the gitlab backend

### `tools.<gitlab:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<gitlab:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<gitlab:…>.version_prefix`

release tag prefix stripped from versions and re-added when resolving tags

### `tools.<gitlab:…>.version_regex`

regex a release tag must match to be listed; versions are still derived from the tag as usual

### `tools.<gitlab:…>.asset_pattern`

glob (\* and ?) matched against release asset names, tera templated; replaces autodetection

### `tools.<gitlab:…>.additional_asset_patterns`

extra archives from the same release extracted over the primary asset, in order

### `tools.<gitlab:…>.matching`

substring an asset name must contain, applied before platform autodetection

### `tools.<gitlab:…>.matching_regex`

regex an asset name must match, applied before platform autodetection

### `tools.<gitlab:…>.no_app`

penalize macos .app bundle assets during autodetection

### `tools.<gitlab:…>.filter_bins`

only expose these binaries on PATH

### `tools.<gitlab:…>.format`

force the archive format instead of detecting it from the file name

### `tools.<gitlab:…>.checksum`

expected artifact digest as algo:hash (sha256, sha512, sha1, md5, blake3)

### `tools.<gitlab:…>.size`

expected artifact size in bytes

### `tools.<gitlab:…>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<gitlab:…>.bin`

executable name (or safe relative path) to install the downloaded binary as

### `tools.<gitlab:…>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<gitlab:…>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<gitlab:…>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<gitlab:…>.api_url`

api base url, for self-hosted gitlab instances

### `tools.<forgejo:…>`

a tool installed with the forgejo backend

### `tools.<forgejo:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<forgejo:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<forgejo:…>.version_prefix`

release tag prefix stripped from versions and re-added when resolving tags

### `tools.<forgejo:…>.version_regex`

regex a release tag must match to be listed; versions are still derived from the tag as usual

### `tools.<forgejo:…>.asset_pattern`

glob (\* and ?) matched against release asset names, tera templated; replaces autodetection

### `tools.<forgejo:…>.additional_asset_patterns`

extra archives from the same release extracted over the primary asset, in order

### `tools.<forgejo:…>.matching`

substring an asset name must contain, applied before platform autodetection

### `tools.<forgejo:…>.matching_regex`

regex an asset name must match, applied before platform autodetection

### `tools.<forgejo:…>.no_app`

penalize macos .app bundle assets during autodetection

### `tools.<forgejo:…>.filter_bins`

only expose these binaries on PATH

### `tools.<forgejo:…>.format`

force the archive format instead of detecting it from the file name

### `tools.<forgejo:…>.checksum`

expected artifact digest as algo:hash (sha256, sha512, sha1, md5, blake3)

### `tools.<forgejo:…>.size`

expected artifact size in bytes

### `tools.<forgejo:…>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<forgejo:…>.bin`

executable name (or safe relative path) to install the downloaded binary as

### `tools.<forgejo:…>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<forgejo:…>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<forgejo:…>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<forgejo:…>.api_url`

api base url, for self-hosted forgejo or gitea instances

### `tools.<ubi:…>`

a tool installed with the ubi backend

### `tools.<ubi:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<ubi:…>.platforms.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<ubi:…>.platforms.<key>.bin_path`

directory in the install containing binaries, used with extract\_all

### `tools.<ubi:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<ubi:…>.platform.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<ubi:…>.platform.<key>.bin_path`

directory in the install containing binaries, used with extract\_all

### `tools.<ubi:…>.exe`

name of the executable inside the archive to extract

### `tools.<ubi:…>.rename_exe`

name to give the extracted executable

### `tools.<ubi:…>.matching`

substring to match release filenames when several match the os and arch

### `tools.<ubi:…>.matching_regex`

regex release filenames must match before os and arch matching

### `tools.<ubi:…>.provider`

forge used to fetch releases and assets

### `tools.<ubi:…>.api_url`

provider api base url, for self-hosted instances

### `tools.<ubi:…>.tag_regex`

regex release tags must match to be listed as versions

### `tools.<ubi:…>.extract_all`

extract every file in the archive instead of only the binary

### `tools.<ubi:…>.bin_path`

directory in the install containing binaries, used with extract\_all

### `tools.<http:…>`

a tool installed with the http backend

### `tools.<http:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<http:…>.platforms.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<http:…>.platforms.<key>.url`

download url, tera templated, http(s) or file://; required for each platform

### `tools.<http:…>.platforms.<key>.headers`

request headers sent with the download, version\_list\_url and checksum\_url requests; values are tera templated (e.g. Bearer {{ env.TOKEN }})

### `tools.<http:…>.platforms.<key>.headers.<key>`

value of this request header; tera templated

### `tools.<http:…>.platforms.<key>.headers_forward`

hosts each header in headers may follow a redirect to; exact hosts or \*.suffix, and a header with no entry is never forwarded

### `tools.<http:…>.platforms.<key>.headers_forward.<key>`

hosts this header may follow a redirect to: exact hosts or \*.suffix

### `tools.<http:…>.platforms.<key>.checksum`

expected artifact digest as algo:hash

### `tools.<http:…>.platforms.<key>.checksum_url`

checksum file or manifest url, used by mise lock without downloading

### `tools.<http:…>.platforms.<key>.size`

expected artifact size in bytes

### `tools.<http:…>.platforms.<key>.format`

force the archive format instead of detecting it from the url

### `tools.<http:…>.platforms.<key>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<http:…>.platforms.<key>.bin`

name for a downloaded raw or compressed single binary (ignored for archives)

### `tools.<http:…>.platforms.<key>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<http:…>.platforms.<key>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<http:…>.platforms.<key>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<http:…>.platforms.<key>.windows_script_interpreter`

windows only: write a .cmd launcher running the raw script with this interpreter

### `tools.<http:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<http:…>.url`

download url, tera templated, http(s) or file://; required for each platform

### `tools.<http:…>.checksum`

expected artifact digest as algo:hash

### `tools.<http:…>.checksum_url`

checksum file or manifest url, used by mise lock without downloading

### `tools.<http:…>.checksum_expr`

expr-lang expression extracting algo:hash from the checksum\_url manifest

### `tools.<http:…>.headers`

request headers sent with the download, version\_list\_url and checksum\_url requests; values are tera templated (e.g. Bearer {{ env.TOKEN }})

### `tools.<http:…>.headers.<key>`

value of this request header; tera templated

### `tools.<http:…>.headers_forward`

hosts each header in headers may follow a redirect to; exact hosts or \*.suffix, and a header with no entry is never forwarded

### `tools.<http:…>.headers_forward.<key>`

hosts this header may follow a redirect to: exact hosts or \*.suffix

### `tools.<http:…>.size`

expected artifact size in bytes

### `tools.<http:…>.format`

force the archive format instead of detecting it from the url

### `tools.<http:…>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<http:…>.bin`

name for a downloaded raw or compressed single binary (ignored for archives)

### `tools.<http:…>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<http:…>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<http:…>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<http:…>.shared_extraction`

share extracted content between installs with identical artifacts

### `tools.<http:…>.windows_script_interpreter`

windows only: write a .cmd launcher running the raw script with this interpreter

### `tools.<http:…>.version_list_url`

url returning the version list (text, json or html)

### `tools.<http:…>.version_regex`

regex extracting versions from the list (first capture group or whole match)

### `tools.<http:…>.version_json_path`

jq-like path extracting versions from json, such as .\[].version

### `tools.<http:…>.version_expr`

expr-lang expression post-processing versions (variables: body, versions)

### `tools.<http:…>.version_date_url`

url holding release dates; defaults to the version\_list\_url body

### `tools.<http:…>.version_date_next_expr`

expr-lang expression returning the url of the next page of version\_date\_url, or nil after the last (variables: body, url)

### `tools.<http:…>.version_date_expr`

expr-lang expression mapping each version to its release date, as a list of \[version, date] pairs or a map of version to date (variables: body, bodies, versions)

### `tools.<s3:…>`

a tool installed with the s3 backend

### `tools.<s3:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<s3:…>.platforms.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<s3:…>.platforms.<key>.url`

s3://bucket/key object url, tera templated; required for each platform

### `tools.<s3:…>.platforms.<key>.checksum`

expected artifact digest as algo:hash

### `tools.<s3:…>.platforms.<key>.size`

expected artifact size in bytes

### `tools.<s3:…>.platforms.<key>.format`

force the archive format when the object key has no usable extension

### `tools.<s3:…>.platforms.<key>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<s3:…>.platforms.<key>.bin`

executable name (or safe relative path) to install the downloaded binary as

### `tools.<s3:…>.platforms.<key>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<s3:…>.platforms.<key>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<s3:…>.platforms.<key>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<s3:…>.platforms.<key>.region`

aws region used for the s3 client

### `tools.<s3:…>.platforms.<key>.endpoint`

custom s3-compatible endpoint url (minio and similar); enables path-style addressing

### `tools.<s3:…>.platforms.<key>.version_list_url`

s3:// url of a manifest file listing versions

### `tools.<s3:…>.platforms.<key>.version_prefix`

s3 key prefix to list objects under for version discovery

### `tools.<s3:…>.platforms.<key>.version_regex`

regex extracting versions from the manifest or object keys

### `tools.<s3:…>.platforms.<key>.version_json_path`

jq-like path extracting versions from json, such as .\[].version

### `tools.<s3:…>.platforms.<key>.version_expr`

expr-lang expression post-processing versions (variables: body, versions)

### `tools.<s3:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<s3:…>.url`

s3://bucket/key object url, tera templated; required for each platform

### `tools.<s3:…>.checksum`

expected artifact digest as algo:hash

### `tools.<s3:…>.size`

expected artifact size in bytes

### `tools.<s3:…>.format`

force the archive format when the object key has no usable extension

### `tools.<s3:…>.strip_components`

leading directory components to strip on extract (auto 1 for single-root archives)

### `tools.<s3:…>.bin`

executable name (or safe relative path) to install the downloaded binary as

### `tools.<s3:…>.rename_exe`

rename the extracted executable: a name, or a table of glob = new name

### `tools.<s3:…>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<s3:…>.bin_path`

directory inside the install that contains binaries (tera templated)

### `tools.<s3:…>.region`

aws region used for the s3 client

### `tools.<s3:…>.endpoint`

custom s3-compatible endpoint url (minio and similar); enables path-style addressing

### `tools.<s3:…>.version_list_url`

s3:// url of a manifest file listing versions

### `tools.<s3:…>.version_prefix`

s3 key prefix to list objects under for version discovery

### `tools.<s3:…>.version_regex`

regex extracting versions from the manifest or object keys

### `tools.<s3:…>.version_json_path`

jq-like path extracting versions from json, such as .\[].version

### `tools.<s3:…>.version_expr`

expr-lang expression post-processing versions (variables: body, versions)

### `tools.<aqua:…>`

a tool installed with the aqua backend

### `tools.<aqua:…>.symlink_bins`

expose only the registry-declared binaries through a filtered bin directory

### `tools.<aqua:…>.libc`

force strict glibc or musl asset selection on linux

### `tools.<aqua:…>.slsa_signer_identity`

expected fulcio certificate subject for slsa provenance; requires slsa\_signer\_issuer

### `tools.<aqua:…>.slsa_signer_issuer`

expected oidc issuer for slsa provenance; requires slsa\_signer\_identity

### `tools.<aqua:…>.vars`

values for aqua registry template variables

### `tools.<aqua:…>.vars.<name>`

value of this aqua registry template variable

### `tools.<cargo:…>`

a tool installed with the cargo backend

### `tools.<cargo:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<cargo:…>.platforms.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<cargo:…>.platforms.<key>.bin`

binary to install when a crate has several (cargo install --bin)

### `tools.<cargo:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<cargo:…>.platform.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<cargo:…>.platform.<key>.bin`

binary to install when a crate has several (cargo install --bin)

### `tools.<cargo:…>.features`

crate features to enable (cargo install --features); skips cargo-binstall

### `tools.<cargo:…>.default-features`

set to false to pass --no-default-features; skips cargo-binstall

### `tools.<cargo:…>.bin`

binary to install when a crate has several (cargo install --bin)

### `tools.<cargo:…>.rename_exe`

rename the installed executable: a name, or a table of glob = new name

### `tools.<cargo:…>.rename_exe.<key>`

new name for the executables matching this glob

### `tools.<cargo:…>.crate`

crate to install when a git repository contains several

### `tools.<cargo:…>.locked`

pass --locked to cargo install (default true)

### `tools.<npm:…>`

a tool installed with the npm backend

### `tools.<npm:…>.allow_builds`

dependency packages whose build scripts are approved; true approves all

### `tools.<npm:…>.trust_policy_excludes`

packages or version ranges exempt from aube's trustPolicy no-downgrade check

### `tools.<npm:…>.allow_low_downloads`

approve the requested package past aube's reputation checks

### `tools.<npm:…>.allow_exotic_deps`

dependencies allowed from non-registry sources (git, file, tarball url); true allows all

### `tools.<npm:…>.aube_args`

extra arguments for aube add --global (only with npm.package\_manager = aube\_cli)

### `tools.<npm:…>.pnpm_args`

extra arguments for pnpm installs (shell-split)

### `tools.<npm:…>.bun_args`

extra arguments for bun installs (shell-split)

### `tools.<npm:…>.npm_args`

extra arguments for npm installs (shell-split)

### `tools.<pypi:…>`

a tool installed with the pypi backend

### `tools.<pypi:…>.extras`

python package extras to install

### `tools.<pypi:…>.with`

additional python requirements installed into the tool environment (requires uv)

### `tools.<pypi:…>.expose`

extra requirements whose executables are exposed (requires uv 0.8.5+)

### `tools.<pypi:…>.dependency_prereleases`

uv prerelease policy for dependency resolution (requires uv)

### `tools.<pypi:…>.python`

mise-managed CPython version for this tool (minor or exact patch)

### `tools.<pypi:…>.package_name`

distribution name for git sources, used to attach extras

### `tools.<pypi:…>.pipx_args`

extra arguments for pipx install (shell-split)

### `tools.<pypi:…>.uvx`

set to false to install with pipx instead of uv

### `tools.<pypi:…>.uvx_args`

extra arguments for uv tool install (shell-split)

### `tools.<pypi:…>.registry_url`

version-listing url containing a {} placeholder for the package name

### `tools.<pypi:…>.registry_index`

how dependency locking uses registry\_url: default replaces PyPI, priority searches it before PyPI

### `tools.<pipx:…>`

a tool installed with the pypi backend

### `tools.<gem:…>`

a tool installed with the gem backend

### `tools.<gem:…>.source`

gem registry http(s) url used to resolve and install this gem

### `tools.<go:…>`

a tool installed with the go backend

### `tools.<go:…>.tags`

build tags passed to go install -tags

### `tools.<conda:…>`

a tool installed with the conda backend

### `tools.<conda:…>.channel`

conda channel to resolve the package from (default conda-forge)

### `tools.<spm:…>`

a tool installed with the spm backend

### `tools.<spm:…>.platforms`

per-platform option overrides keyed by \<os>-\<arch>, such as linux-x64 or macos-arm64; or per-platform option overrides

### `tools.<spm:…>.platforms.<key>`

options for this platform, overriding the same options set for the whole tool

### `tools.<spm:…>.platforms.<key>.provider`

git host used for release info and assets

### `tools.<spm:…>.platforms.<key>.api_url`

provider api url, for self-hosted instances

### `tools.<spm:…>.platforms.<key>.artifactbundle`

true requires an artifactbundle, false forces a source build, unset tries the bundle first

### `tools.<spm:…>.platforms.<key>.artifactbundle_asset`

specific \*.artifactbundle.zip release asset to use

### `tools.<spm:…>.platform`

per-platform option overrides (alias of platforms)

### `tools.<spm:…>.provider`

git host used for release info and assets

### `tools.<spm:…>.api_url`

provider api url, for self-hosted instances

### `tools.<spm:…>.artifactbundle`

true requires an artifactbundle, false forces a source build, unset tries the bundle first

### `tools.<spm:…>.artifactbundle_asset`

specific \*.artifactbundle.zip release asset to use

### `tools.<spm:…>.filter_bins`

executable products to install

### `tools.<spm:…>.install_command`

command run in the checkout instead of swift build; incompatible with filter\_bins

### `tools.<packslip:…>`

a tool installed with the packslip backend

### `tools.<packslip:…>.variant`

publisher-declared alternative build to select, such as fips

### `tools.<packslip:…>.pubkey`

minisign public key line or path to a .pub file pinning the signer

### `tools.<packslip:…>.identity`

exact keyless signer certificate identity

### `tools.<packslip:…>.identity_prefix`

keyless signer certificate identity prefix

### `tools.<packslip:…>.workflow`

repository workflow file, or a list of them, that signs releases on a tag (or on the ref a name ends with, as in release.yml@refs/heads/main); github.com projects only, not valid with pubkey, identity, identity\_prefix, or issuer

### `tools.<packslip:…>.list_identity_prefix`

separate identity prefix for the vendor release list; needs an issuer, not valid with pubkey

### `tools.<packslip:…>.issuer`

oidc issuer for the keyless signer

### `tools.<packslip:…>.allow_unlogged`

accept key-signed bundles without transparency-log evidence

### `tools.<packslip:…>.ignore_requirements`

install despite confirmed host requirement failures

### `tools.<packslip:…>.trust`

vendor exempts this tool from configured packslip stampers

### `tools.<spinel:…>`

a tool installed with the spinel backend

### `tools.<spinel:…>.entrypoint`

ruby file to compile, relative to the repository root (default main.rb)

### `tools.<spinel:…>.bin`

name of the executable written to bin/ (default: the repository name)

### `tools.<spinel:…>.tag_prefix`

text before the version in each git tag, such as v

### `tools.<spinel:…>.source_ref`

full 40-character commit SHA to build instead of the tag

### `tools.<spinel:…>.spinel`

spinel compiler to run (default: spinel on PATH)

### `tools.python`

the core python tool

### `tools.python.patch_sysconfig`

patch sysconfig paths of precompiled python after install (default true, unix only)

### `tools.python.virtualenv`

virtualenv path to activate; use env.\_.python.venv instead

### `tools.java`

the core java tool

### `tools.java.release_type`

jdk metadata channel: ga (default) or ea for early access builds

### `tools.rust`

the core rust tool

### `tools.rust.profile`

rustup profile to install, such as minimal, default or complete

### `tools.rust.components`

rustup components to install

### `tools.rust.targets`

rustup targets to install, for cross-compilation

### `tools.rust.mr_boxington`

wrap cargo with mr boxington (mbx); needs mr-boxington in tools (default false)

### `tools.dotnet`

the core dotnet tool

### `tools.dotnet.runtime`

install a shared runtime instead of the sdk; the version is the runtime version

### `tools.dotnet-core`

the core dotnet tool

### `tools.dotnet-core.runtime`

install a shared runtime instead of the sdk; the version is the runtime version

## `vars`

variables to set for use in config templates

### `vars._`

vars modules

### `vars._.file`

files to load variables from (dotenv, json, yaml, or toml)

### `vars._.source`

bash scripts to run, taking the variables they export

### `vars.<name>`

a variable: its value, or a table with `value`, `default`, `required` or `age`

### `vars.<name>.value`

value of variable

### `vars.<name>.tools`

load tools before resolving

### `vars.<name>.redact`

redact the value from logs

### `vars.<name>.default`

fallback value to use when the environment variable is unset or empty

### `vars.<name>.prompt`

question to ask once, on a machine with no saved answer, when a command is run with a prompt flag such as mise bootstrap --prompt-vars

### `vars.<name>.required`

fail unless the process environment, a higher-precedence config file, or a saved answer supplies a value: true, or help text shown in the error

### `vars.<name>.age`

age-encrypted value (simplified format); or \[experimental] age-encrypted value (complex format)

### `vars.<name>.age.value`

age-encrypted value

### `vars.<name>.age.format`

compression format for the encrypted value

### `vars.<name>.age.tools`

load tools before resolving

### `vars.<name>.age.redact`

redact the value from logs

## `watch_files`

files to watch for changes and run scripts when modified

### `watch_files[].run`

script to run when file changes

### `watch_files[].shell`

inline shell command used to run script when file changes

### `watch_files[].patterns`

patterns to watch for

### `watch_files[].task`

mise task to run when file changes

## `wrappers`

commands that intercept a binary name before delegating to the active toolset

### `wrappers.<command>`

command to run

### `wrappers.<command>.command`

command to run

### `wrappers.<command>.args`

arguments inserted before the intercepted command's arguments

### `wrappers.<command>.env`

environment variables set for the wrapper

### `wrappers.<command>.env.<NAME>`

value of this environment variable

[Edit this page](https://github.com/jdx/mise/edit/main/docs/configuration/reference.md)

Last updated:

Pager

[Previous pagemise.toml](https://mise.jdx.dev/configuration.html)

[Next pagemise.toml playground](https://mise.jdx.dev/configuration/playground.html)

MIT License·Copyright © 2026· [![](https://github.com/jdx.png?size=96)jdx.dev](https://jdx.dev)