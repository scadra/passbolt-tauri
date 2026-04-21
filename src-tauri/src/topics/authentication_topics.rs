define_topics!(AuthenticationTopics {
    VERIFY_PASSPHRASE => "passbolt.auth.verify-passphrase",
    DESKTOP_AUTHENTICATE => "passbolt.auth.login",
    AFTER_LOGIN => "passbolt.auth.after-login",
    IS_AUTHENTICATED => "passbolt.auth.is-authenticated",
    VERIFY_SERVER => "passbolt.auth.verify-server-key",
    GET_SERVER_KEY => "passbolt.auth.get-server-key",
    SET_SERVER_KEY => "passbolt.auth.replace-server-key",
    LOG_OUT => "passbolt.auth.logout",
    POST_LOGIN_REDIRECT => "passbolt.auth.post-login-redirect",
    REQUIRE_MFA => "passbolt.auth.redirect-to-mfa",
    DESKTOP_AUTHENTICATE_DESKTOP => "passbolt.desktop.authenticate",
});
