define_topics!(UserTopics {
    FIND_LOGGED_IN_USER => "passbolt.users.find-logged-in-user",
    UPDATE_LOCALSTORAGE => "passbolt.users.update-local-storage",
    GET_ALL => "passbolt.users.get-all",
    CREATE => "passbolt.users.create",
    UPDATE => "passbolt.users.update",
    DELETE_DRY_RUN => "passbolt.users.delete-dry-run",
    DELETE => "passbolt.users.delete",
    RESEND_INVITE => "passbolt.users.resend-invite",
    UPDATE_SECURITY_TOKEN => "passbolt.users.update-security-token",
    UPDATE_PRIVATE_KEY => "passbolt.user.update-private-key",
});
