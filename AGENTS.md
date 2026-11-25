# AGENTS.md

Notes for coding agents working on Rummo. If the maintainer tells you something different from this file, do what they say, with one exception: agents never open pull requests, even when asked.

## What Rummo is

Rummo is a Rust plugin for the [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) Minecraft server. It adds MMO-style features: skills, abilities.

- It runs as a **WebAssembly component** (`wasm32-wasip2`) inside Pumpkin's plugin runtime. Pumpkin loads `plugins/rummo.wasm` from the server directory.
- Check `Cargo.lock` for the exact `pumpkin-plugin-api` version (`0.2.1+26.3-26.51` at present). Use the matching crate docs or Cargo registry source for signatures. Pumpkin's main branch can expose a different API.

## Commands

The workspace's default member is `tools/run-server`, so a bare `cargo build` does **not** build the plugin.

`rust-toolchain.toml` pins Rust `1.100.0` and installs the `wasm32-wasip2` target. If the formatting or lint components are missing, install them for the pinned toolchain with `rustup component add rustfmt clippy`.

| Task | Command |
| --- | --- |
| Build the plugin | `cargo build-plugin` (alias for `build -p rummo --lib --target wasm32-wasip2`) |
| Unit tests | `cargo test -p rummo` |
| Lint | `cargo clippy -p rummo --target wasm32-wasip2` |
| Format | `cargo fmt --all` |
| Build, install and start a server | `cargo run` (uses `server/pumpkin.exe` on Windows, `server/pumpkin` elsewhere, or `PUMPKIN_EXE`) |

The launcher copies the build to `plugins/rummo.wasm` next to the chosen executable and starts Pumpkin in that directory. The rule below for writes to `server/` applies to the default launch.

Before you say a change is done, `cargo fmt --all`, the Clippy command, `cargo test -p rummo` and `cargo build-plugin` must all pass. Leave in-game checks to the maintainer and report which behaviour still needs checking.

## Layout

- `src/lib.rs` wires the plugin together in `on_load`. `on_unload` clears boss bar state and flushes skill data through `SkillManager::shutdown`.
- `src/commands.rs` registers `/rummo`, the mining status command, and staff commands for editing experience and levels. Permission nodes are declared alongside the commands.
- `src/events/` contains the Pumpkin event handlers. `right_click_air_event.rs` handles active ability input. `interact_event.rs` is an empty placeholder.
- `src/skills/skill_manager.rs` defines `SkillType` and the global `SkillManager` (`add_exp`, `set_exp`, `get_skill`). `src/skills/experience.rs` defines `ExperienceConfig` and the levelling maths. Experience is progress within the current level.
- `src/abilities/` contains ability definitions, the registry, runtime cooldown and expiration state, and active ability effects such as `mining/super_breaker.rs`.
- `src/storage/cache.rs` keeps progress and pending writes in memory. `src/storage/sqlite.rs` implements **redb** persistence in `skills.redb`. Autosave runs every `20 * 30` ticks, with a final save on unload.
- `src/ui/` manages per-player skill progress bars. `bossbar_scheduler.rs` updates them every 4 ticks and hides them after 5 seconds without an experience update. `bossbar_render.rs` formats the text.
- `src/config.rs` loads TOML from the plugin data folder and writes embedded defaults from `config/` when files are missing. Existing config files are read as-is.
- `tools/run-server/` is the native development launcher.


## How-to recipes

### Add a new skill

1. Add a variant to `SkillType` and to both `id()` and `from_id()`. The id is stored in the database: never rename or reuse an existing id.
2. Add its display name in `SkillType::display_name()`.
3. Put passive gameplay effects in `src/skills/<skill>/`, modelled on `mining/`, and wire them in `lib.rs::on_load` as needed. `SkillManager` stores one level-change handler, so extend its dispatch if a second skill needs level-change effects.
4. Award XP from an event handler with `SkillManager::add_exp`. Add a descriptive skill command in `src/commands.rs` if the feature requires one.

### Add an active ability

1. Add an `AbilityId` variant and its definition under `[abilities.<snake_case_id>]` in `config/abilities.toml`.
2. Put the effects under `src/abilities/<skill>/` and wire the activation input. Use a distinct attribute modifier ID for each effect.
3. Add the activation arm in `active_ability_manager.rs` and, for a timed effect, the matching expiration arm in `ability_lifecycle_service.rs`. Keep orchestration and heap handling in the shared managers.

### Add an event handler

Create `src/events/<name>_event.rs` with a `register(context, ...)` function and a struct implementing `EventHandler<...>`. Use the closest existing handler as a model, choose the priority and blocking flag deliberately, and return `data` early if `data.cancelled` applies to the event. Use a blocking registration when the host must apply edits to the returned event. Declare the module in `events/mod.rs` and register it from `on_load`.

### Add a config file

Follow `MiningEffects::load` in `src/skills/mining/effects.rs`. Embed `config/<name>.toml` with `include_str!`, call `crate::config::load`, and validate the parsed values. Include the path in validation errors and comment each key in the default file. Handle compatibility explicitly when adding required fields, since installed configs are never overwritten.

## Rules

### Correctness

- Persisted formats are a contract: do not change the redb table definition, key layout or skill ids without a migration and the maintainer's approval.
- The plugin is single-threaded Wasm, and **host calls can yield** to other callbacks. Release `Mutex` guards before host calls, including player lookups, boss bar calls, scheduling, cancellation, and filesystem operations. See `ability_lifecycle_service.rs` and `storage/mod.rs`.
- Use `schedule_repeating_task` for periodic work and `schedule_delayed_task` for a deadline. Do not use Tokio, Rayon, threads or blocking sleeps.
- Keep `[profile.dev.package.rummo]` and `pumpkin-plugin-api` at `opt-level = 2`; Pumpkin cannot load unoptimised builds.
- Validate config at load and fail `on_load` with a clear error rather than falling back silently.

### Errors and logging

- No `unwrap`, `panic!`, `todo!`, `println!` or `eprintln!` in plugin code. `expect("<name> lock")` is allowed only for `Mutex::lock` poisoning. Tests may `unwrap`.
- Log with `tracing` (`error!` for failures you swallow, `info!` sparingly).
- Player-facing text is Chinese; log messages, errors and code comments are English.

### Style
- Do not make simple things complicated: If it's not obvious, it's not simple.
- Copy the closest existing code: how it registers, handles errors and orders imports (std, external crates, then `crate`/`super`).
- Name magic numbers as constants (see `FLUSH_INTERVAL_TICKS`, `HIDE_AFTER`). Tick values are written as `20 * seconds`.
- Keep changes as small as the task. No abstractions, traits, config options or feature flags for hypothetical future use; three similar lines are fine.
- Search for an existing helper first (`player_uuid`, `block_id`, ...). If the same logic appears twice, extract it.
- Use the smallest visibility that works (`pub(super)` / `pub(crate)`).
- No new clippy warnings. Do not fix unrelated existing ones in the same change.
- Doc comments: one plain summary sentence on items whose behaviour isn't obvious; no comments that narrate the next line.
- Add or update a unit test when you change pure logic (levelling maths, cache, config parsing, or expiration and wakeup decisions).

### Things not to do without asking

- Add dependencies, bump `pumpkin-plugin-api` or the toolchain, or edit `Cargo.lock` by hand.
- Rename public items or files, or reorganise modules.
- Touch `server/` or anything git-ignored.
