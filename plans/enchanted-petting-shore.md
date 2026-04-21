# Plan : Rebuild src-tauri/src/ depuis main.rs

## Context

Reset du projet. On garde :
- `main.rs` (tel quel — on le met a jour pour compiler)
- `src/isolation/` (index.js + index.html)
- `topics/` (33 modules + mod.rs)
- `plugins/mod.rs` (reference `navigation` qui n'existe plus)
- `Cargo.toml`, `tauri.conf.json`, `apps/` JS inchanges

On re-cree tout ce que `main.rs` reference et qui a ete supprime.

## Fichiers manquants (references par main.rs)

```rust
use passbolt_app::AppResult;                    // → lib.rs
use passbolt_app::config;                       // → config.rs
use passbolt_app::state::AppState;              // → state.rs
use passbolt_app::services::local_folder_service::LocalFolderService;   // → services/
use passbolt_app::services::protocol_service::ProtocolService;
use passbolt_app::services::rendered_topic_service::RenderedTopicService;
use passbolt_app::services::background_topic_service::BackgroundTopic;
use passbolt_app::models::ipc_model::IPC;       // → models/
use passbolt_app::models::messaging::AllowedTopics;
```

+ `plugins::navigation` reference par `plugins/mod.rs`

## Fichiers a creer

| Fichier | Role |
|---------|------|
| `lib.rs` | Exports + `define_topics!` macro + `AppResult` type |
| `error.rs` | `AppError` enum |
| `config.rs` | Constantes (labels, events, dimensions) |
| `state.rs` | `AppState` + `CredentialCache` — **avec `load_into_cache` appele au startup** |
| `models/mod.rs` | Module declarations |
| `models/ipc_model.rs` | `IPC` struct |
| `models/messaging.rs` | `AllowedTopics` constants |
| `models/account_model.rs` | `AccountMetaData`, `AccountSecret`, `AccountKit` |
| `models/asset_model.rs` | `rust-embed` structs |
| `services/mod.rs` | Module declarations |
| `services/local_folder_service.rs` | HTML generation + CSP + asset serving |
| `services/protocol_service.rs` | `passbolt://` handler — **avec routes workspace** |
| `services/credential_service.rs` | OS keyring CRUD |
| `services/rendered_topic_service.rs` | Topic whitelist + forwarding |
| `services/background_topic_service.rs` | Topic routing + handlers — **avec workspace transition + logout** |
| `plugins/navigation.rs` | Navigation plugin |

## Changements vs l'ancienne version

1. **`main.rs` setup()** : appeler `CredentialService::load_into_cache()` + passer `has_account` a `initialize_app()`
2. **`initialize_app()`** : parametre `has_account: bool` → naviguer vers `/background-auth` + `/app-auth` si true
3. **`protocol_service.rs`** : ajouter routes `/background-workspace` + `/app-workspace`
4. **`background_topic_service.rs`** : implementer `AFTER_LOGIN` → `redirect_to_workspace()`, `LOG_OUT` → navigate to auth
5. **`account_model.rs`** : modeles propres avec serde rename corrects (basé sur le fichier que tu as montre)

## Verification

```bash
cargo build --manifest-path=src-tauri/Cargo.toml
cargo test --manifest-path=src-tauri/Cargo.toml
npm run tauri:dev
```
