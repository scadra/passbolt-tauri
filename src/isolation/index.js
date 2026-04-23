window.__TAURI_ISOLATION_HOOK__ = (payload) => {
    try {
        // 1. Command whitelist — only allow Tauri event system commands + opener.
        // Using Set.has() instead of Array.includes() to prevent prototype pollution bypass.
        const allowedCommands = new Set([
            'plugin:event|emit',
            'plugin:event|listen',
            'plugin:event|unlisten',
            'plugin:opener|open_url',
        ]);
        if (!allowedCommands.has(payload.cmd)) {
            console.error('Isolation: Blocked command:', payload.cmd);
            return null;
        }

        // 2. Event name validation — for emit commands, only allow known IPC channels.
        // These match the constants in src-tauri/src/config.rs (lines 12-15).
        if (payload.cmd === 'plugin:event|emit') {
            const allowedEventNames = new Set([
                'rendered-ipc-message',
                'background-ipc-message',
                'rendered-ipc-forward',
                'background-ipc-forward',
            ]);
            const eventName = payload?.payload?.event;
            if (typeof eventName !== 'string' || !allowedEventNames.has(eventName)) {
                console.error('Isolation: Blocked emit with invalid event name:', eventName);
                return null;
            }
        }

        // 3. Payload shape validation — for emit commands, enforce that the IPC envelope
        // is a non-null object with a non-empty topic string.
        // This matches the IpcMessage struct in src-tauri/src/models/ipc_message.rs.
        //
        // IMPORTANT: Every bypass path (undefined, null, array) is explicitly blocked.
        // The Rust layer also validates via serde deserialization, but the hook must not
        // give a false sense of security by silently passing invalid payloads.
        if (payload.cmd === 'plugin:event|emit') {
            const innerPayload = payload?.payload?.payload;

            // The IPC envelope must exist — undefined/null payloads are not valid IPC messages
            if (innerPayload === undefined || innerPayload === null) {
                console.error('Isolation: Blocked emit with missing payload');
                return null;
            }

            // Parse if stringified
            let parsed;
            try {
                parsed = typeof innerPayload === 'string' ? JSON.parse(innerPayload) : innerPayload;
            } catch (parseError) {
                console.error('Isolation: Blocked emit with unparseable payload');
                return null;
            }

            // Must be a plain object (not null, not array, not primitive)
            if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
                console.error('Isolation: Blocked emit with non-object payload');
                return null;
            }

            // Must have a non-empty topic string
            if (typeof parsed.topic !== 'string' || parsed.topic.length === 0) {
                console.error('Isolation: Blocked emit with missing or empty topic');
                return null;
            }
        }

        // Note on sender validation: in Tauri v2's isolation pattern, the sender
        // identity is not available in the isolation hook payload. Sender validation
        // is handled at the Rust layer — ipc_listener_service.rs registers per-webview
        // listeners using webview.listen() which scopes events to their source webview,
        // preventing cross-webview impersonation.

        return payload;
    } catch (error) {
        console.error('Isolation: Hook error, blocking message:', error);
        return null;
    }
}
