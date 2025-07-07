// src/models/account_model.rs

// src/models/account_model.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecurityToken {
    pub token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountMetaData {
    pub domain: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "username")]
    pub user_name: String,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    #[serde(rename = "securityToken")]
    pub security_token: SecurityToken,
    #[serde(rename = "serverPublicArmoredKey")]
    pub server_public_armored_key: String,
    #[serde(rename = "userPublicArmoredKey")]
    pub user_public_armored_key: String,
    pub theme: String,
    pub locale: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfiguration {
    #[serde(rename = "renderedUrl")]
    pub rendered_url: String,
    #[serde(rename = "backgroundUrl")]
    pub background_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountSecret {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "userPrivateArmoredKey")]
    pub user_private_armored_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountKit {
    #[serde(rename = "accountMetaData")]
    pub account_meta_data: AccountMetaData,
    #[serde(rename = "accountSecret")]
    pub account_secret: AccountSecret,
}