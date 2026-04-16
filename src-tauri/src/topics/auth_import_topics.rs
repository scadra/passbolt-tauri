define_topics!(AuthImportTopics {
    VALIDATE_ACCOUNT_KIT => "passbolt.background.verify-account-kit",
    VERIFY_PASSPHRASE => "passbolt.auth-import.verify-passphrase",
    IMPORT_ACCOUNT => "passbolt.auth-import.import-account",
    SAVE_ACCOUNT => "passbolt.background.save-account",
    IMPORT_SIGN_IN => "passbolt.auth-import.sign-in",
});
