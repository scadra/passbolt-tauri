# Routing

Two topic services decide what to do with each incoming message: validate, intercept, forward, or drop.

## RenderedTopicService

File: `services/rendered_topic_service.rs` (71 lines)

Handles messages **from** the Rendered webview. Owns an `AllowedTopics` instance.

```
Message from Rendered
        |
        v
  topic in AllowedTopics? ----NO----> warn + DROP
        |
       YES
        v
  requestId present?
        |
       YES --> validate UUID regex (line 66-70)
        |       invalid? --> warn + DROP
        |       valid? --> register in ipc.request_ids
        v
  match topic:
        |
        +-- RENDERED_READY
        |   -> rendered_handlers::handle_rendered_ready()
        |
        +-- RENDERED_ASSET_FETCH
        |   -> rendered_handlers::handle_asset_fetch()
        |
        +-- RbacTopics::FIND_ME
        |   -> rendered_handlers::handle_rbac_request()
        |      (tracks as pending request, then forwards)
        |
        +-- FolderTopics::MOVE_STRATEGY_REQUEST
        |   SecretTopics::PASSPHRASE_REQUEST
        |   ProgressTopics::* (4 variants)
        |   -> handlers::forward_to_background()
        |
        +-- everything else
            -> handlers::forward_to_background()
```

UUID validation regex: `^[0-9a-f]{8}-[0-9a-f]{4}-[0-5][0-9a-f]{3}-[089ab][0-9a-f]{3}-[0-9a-f]{12}$`

## BackgroundTopicService

File: `services/background_topic_service.rs` (53 lines)

Handles messages **from** the Background webview. No whitelist check (Background is trusted).

```
Message from Background
        |
        v
  match topic:
        |
        +-- BACKGROUND_READY
        |   -> bg::handle_background_ready()
        |
        +-- BACKGROUND_LISTENER_READY
        |   -> bg::handle_listener_ready()
        |
        +-- AuthenticationTopics::AFTER_LOGIN
        |   -> bg::handle_after_login()
        |
        +-- AuthenticationTopics::REQUIRE_MFA
        |   -> bg::handle_require_mfa()
        |
        +-- AuthenticationTopics::LOG_OUT
        |   -> bg::handle_logout()
        |
        +-- SAVE_ACCOUNT
        |   -> bg::handle_save_account()
        |
        +-- BACKGROUND_AUTHENTICATION_ERROR
        |   -> bg::handle_authentication_error()
        |
        +-- BACKGROUND_SET_THEME | SET_LOCALE | SET_SECURITY_TOKEN
        |   -> bg::handle_vault_write()
        |
        +-- BACKGROUND_ROTATE_KEY
        |   -> bg::handle_rotate_key()
        |
        +-- LocalStorageTopics (update | delete | clear)
        |   -> handlers::buffer_or_forward_to_rendered()
        |      (remap topic prefix: background -> rendered)
        |
        +-- BACKGROUND_CLIPBOARD
        |   -> bg::handle_clipboard()
        |
        +-- BACKGROUND_DOWNLOAD_FILE
        |   -> bg::handle_download_file()  (stub/no-op)
        |
        +-- BACKGROUND_GET_COOKIE
        |   -> bg::handle_get_cookie()
        |
        +-- BACKGROUND_HTTP_FETCH
        |   -> bg::handle_http_fetch()
        |
        +-- _ (catch-all)
            -> bg::handle_response_forward()
               checks if topic is a registered requestId
               if yes: consume + inject RBAC denies if needed + forward
               if no: forward to rendered as-is
```

## Key Difference

- **RenderedTopicService** validates every message against the whitelist (Rendered is user-facing, could be compromised)
- **BackgroundTopicService** does NOT whitelist-check (Background is trusted, runs only our code)

## Topic Remapping

When localStorage events flow Background -> Rendered, the topic prefix is rewritten in `handlers.rs:162-174`:

```
passbolt.background.localstorage-update  ->  passbolt.rendered.localstorage-update
passbolt.background.localstorage-delete  ->  passbolt.rendered.localstorage-delete
passbolt.background.localstorage-clear   ->  passbolt.rendered.localstorage-clear
```

## Response Detection

The catch-all `handle_response_forward()` in BackgroundTopicService uses `ipc.proceed_request_id(topic)` to check if the topic matches a registered requestId. If yes, it's a response. `proceed_request_id` is one-shot: it consumes the ID, preventing replay.

If the response corresponds to a pending RBAC request (`RbacTopics::FIND_ME`), desktop-specific denials are injected via `rbac_service::inject_desktop_denies()`.
