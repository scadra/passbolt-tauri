window.__TAURI_ISOLATION_HOOK__ = (payload) => {
    try {
        // 1. Command whitelist — only allow Tauri event system commands
        const allowedCommands = [
            'plugin:event|emit',
            'plugin:event|listen',
            'plugin:event|unlisten',
        ];
        if (!allowedCommands.includes(payload.cmd)) {
            console.error('Isolation: Blocked command:', payload.cmd);
            return null;
        }

        // 2. Event name validation — for emit commands, only allow known IPC channels.
        // These match the constants in src-tauri/src/config.rs (lines 12-15).
        if (payload.cmd === 'plugin:event|emit') {
            const allowedEventNames = [
                'rendered-ipc-message',
                'background-ipc-message',
                'rendered-ipc-forward',
                'background-ipc-forward',
            ];
            const eventName = payload?.payload?.event;
            if (typeof eventName !== 'string' || !allowedEventNames.includes(eventName)) {
                console.error('Isolation: Blocked emit with invalid event name:', eventName);
                return null;
            }
        }

        // 3. Payload shape validation — for emit commands, verify the IPC envelope
        // has the expected structure (object with a non-empty topic string).
        // This matches the IpcMessage struct in src-tauri/src/models/ipc_message.rs.
        if (payload.cmd === 'plugin:event|emit') {
            const innerPayload = payload?.payload?.payload;
            if (innerPayload !== undefined && innerPayload !== null) {
                let parsed;
                try {
                    parsed = typeof innerPayload === 'string' ? JSON.parse(innerPayload) : innerPayload;
                } catch (parseError) {
                    console.error('Isolation: Blocked emit with unparseable payload');
                    return null;
                }
                if (parsed !== null && typeof parsed === 'object' && !Array.isArray(parsed)) {
                    if (typeof parsed.topic !== 'string' || parsed.topic.length === 0) {
                        console.error('Isolation: Blocked emit with missing or empty topic');
                        return null;
                    }
                }
            }
        }

        // Note on sender validation: in Tauri v2's isolation pattern, the sender
        // identity is not available in the isolation hook payload. Sender validation
        // is handled at the Rust layer — ipc_listener_service.rs listens on separate
        // event channels (rendered-ipc-message vs background-ipc-message) which are
        // bound to specific webview contexts by Tauri's capability system.

        return payload;
    } catch (error) {
        console.error('Isolation: Hook error, blocking message:', error);
        return null;
    }
}
