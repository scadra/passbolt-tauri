define_topics!(MfaTopics {
    HAS_POSPONE_MFA_INVITATION => "passbolt.mfa-policy.has-user-postponed-user-setting-invitation",
    POSPONE_MFA_INVITATION => "passbolt.mfa-policy.postpone-user-setting-invitation",
    GET_POLICY => "passbolt.mfa-policy.get-policy",
    GET_MFA_SETTINGS => "passbolt.mfa-policy.get-mfa-settings",
    VERIFY_PROVIDER => "passbolt.mfa-setup.verify-provider",
    REMOVE_TOTP => "passbolt.mfa-setup.remove-provider",
    VERIFY_TOTP_CODE => "passbolt.mfa-setup.verify-totp-code",
    VERIFY_YUBIKEY_CODE => "passbolt.mfa-setup.verify-yubikey-code",
    GET_MFA_SETUP_TOTP_CODE => "passbolt.mfa-setup.get-totp-code",
    DISABLE_MFA_FOR_USER => "passbolt.mfa.disable-for-user",
});
