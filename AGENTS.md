# AGENTS.md

Notes for coding agents working on Rummo. If the maintainer tells you something different from this file, do what they say, with one exception: agents never open pull requests, even when asked.

## What Rummo is

Rummo is a plugin for the [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) Minecraft server, written in Rust. It adds MMO-style features: skills with levels and experience (mining is implemented), with weapon reinforcement planned.

- It compiles to a **WebAssembly component** (`wasm32-wasip2`) that Pumpkin loads from `server/plugins/rummo.wasm`. It is not a native library and does not run inside Pumpkin's Tokio runtime.
- The only host API is `pumpkin-plugin-api` 0.2.x: https://docs.rs/pumpkin-plugin-api/0.2.1/ . Check the docs for the pinned version before calling an API; do not guess signatures from Pumpkin's main repo, which differ.

## Commands

The workspace's default member is `tools/run-server`, so a bare `cargo build` does **not** build the plugin.

| Task | Command |
| --- | --- |
| Build the plugin | `cargo build-plugin` (alias for `build -p rummo --lib --target wasm32-wasip2`) |
| Unit tests | `cargo test -p rummo` |
| Lint | `cargo clippy -p rummo --target wasm32-wasip2` |
| Format | `cargo fmt --all` |
| Build, install and start a server | `cargo run` (needs `server/pumpkin` or `PUMPKIN_EXE`) |

Before you say a change is done, `cargo fmt --all`, the clippy command, `cargo test -p rummo` and `cargo build-plugin` must all pass. You cannot test in-game; say which behaviour still needs a manual check on a server.

## Layout

- `src/lib.rs`: plugin entry. `on_load` wires everything together in order; `on_unload` saves data.
- `src/commands.rs`: `/rummo` command and its permission node.
- `src/events/`: one file per Pumpkin event handler (`*_event.rs`). `interact_event.rs` is an empty placeholder.
- `src/skills/skill_manager.rs`: `SkillType`, levelling maths (`ExperienceConfig`) and the global `SkillManager` (`addExp` / `setExp` / `getSkill`).
- `src/skills/<skill>/`: per-skill gameplay effects (e.g. `mining/effects.rs` changes block-break speed).
- `src/storage/`: `cache.rs` is the in-memory cache with pending writes; `sqlite.rs` is actually **redb** (`skills.redb`), not SQLite. Autosave every 30 s plus a final save on unload.
- `src/ui/`: per-player boss bar showing skill progress (`manager` holds state, `scheduler` runs every 4 ticks, `render` builds the text).
- `config/*.toml`: default configs embedded with `include_str!` and written to the plugin data folder on first run. Players' servers keep their own copies, so changing a default does not change existing servers.
- `tools/run-server/`: host-side dev launcher. It is a normal native binary, not part of the plugin.

Data flow for mining: `BlockBreakEvent` → `MiningEvent` looks up XP from `mining_experience.toml` → `SkillManager::addExp` → `SkillStorage` cache (saved later) → `BossBarManager::experience_updated` → scheduler renders the bar.

## How-to recipes

### Add a new skill

1. Add a variant to `SkillType` and to both `id()` and `from_id()`. The id is stored in the database: never rename or reuse an existing id.
2. Add its display name in `ui/bossbar_render.rs::presentation`.
3. Put gameplay effects in `src/skills/<skill>/`, modelled on `mining/`, and register them in `lib.rs::on_load`.
4. Award XP from an event handler with `SkillManager::addExp`.

### Add an event handler

Create `src/events/<name>_event.rs` with a `register(context, ...)` function and a struct implementing `EventHandler<...>`; copy `player_join_event.rs`. Return `data` early if `data.cancelled`. Declare it in `events/mod.rs` and call `register` from `on_load`.

### Add a config file

Follow `skills/mining/effects.rs::MiningEffects::load`: embed `config/<name>.toml` with `include_str!`, write it if missing, parse with `toml`, validate every value, and return errors as `format!("Cannot ... {}: {error}", path.display())`. Comment each key in the default file.

## Rules

### Correctness

- Persisted formats are a contract: do not change the redb table definition, key layout or skill ids without a migration and the maintainer's approval.
- The plugin is single-threaded Wasm, but **host calls can yield** and let other callbacks run. Never hold a `Mutex` guard across a call into `pumpkin-plugin-api` (sending messages, boss bar calls, player lookups, filesystem). Copy what you need out of the lock first, as `bossbar_manager.rs` and `storage/mod.rs` do.
- Do not use Tokio, Rayon, threads or blocking sleeps; they are unavailable or stall the server. Use `context.schedule_repeating_task` for periodic work.
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
- Add or update a unit test when you change pure logic (levelling maths, cache, config parsing).

### Things not to do without asking

- Add dependencies, bump `pumpkin-plugin-api` or the toolchain, or edit `Cargo.lock` by hand.
- Rename public items or files, or reorganise modules.
- Touch `server/` or anything git-ignored.
