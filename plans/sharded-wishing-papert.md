# Fix locale 404 — path double

## Context

Le `loadingPath="/apps/rendered-webview/dist/locales/{{lng}}/{{ns}}.json"` est
resolu comme relatif par WKWebView (custom protocol scheme ne traite pas `/` comme absolu).
La page est a `/apps/rendered-webview/index-workspace.html` → path double :
`/apps/rendered-webview/apps/rendered-webview/dist/locales/...`

## Fix

Changer le `loadingPath` en path relatif dans `AppWorkspace.js` et `AppAuthentication.js` :
- Avant : `/apps/rendered-webview/dist/locales/{{lng}}/{{ns}}.json`
- Apres : `dist/locales/{{lng}}/{{ns}}.json`

La page est deja a `/apps/rendered-webview/` donc le relatif resout correctement.

## Fichiers

| Fichier | Changement |
|---------|-----------|
| `apps/rendered-webview/src/AppWorkspace.js` | loadingPath relatif |
| `apps/rendered-webview/src/AppAuthentication.js` | loadingPath relatif |
