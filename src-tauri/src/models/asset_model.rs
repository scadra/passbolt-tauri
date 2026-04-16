use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../src/isolation/"]
#[exclude = "test/*"]
#[exclude = "node_modules/*"]
pub struct IsolationAssets;

#[derive(Embed)]
#[folder = "../apps/rendered-webview/"]
#[exclude = "test/*"]
#[exclude = "node_modules/*"]
pub struct RenderedAssets;

#[derive(Embed)]
#[folder = "../apps/background-webview/"]
#[exclude = "test/*"]
#[exclude = "node_modules/*"]
pub struct BackgroundAssets;