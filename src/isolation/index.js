window.__TAURI_ISOLATION_HOOK__ = (payload) => {
    try {
        const allowedCommands = [
            'plugin:event|emit',
            'plugin:event|listen',
            'plugin:event|unlisten',
        ];
        if (!allowedCommands.includes(payload.cmd)) {
            console.error('Isolation: Blocked command:', payload.cmd);
            return null;
        }

        //@todo check if the message is allowed
        //@todo check if the message is valid
        //@todo check the sender


        return payload;
    } catch (error) {
        console.log(error)
    }
}