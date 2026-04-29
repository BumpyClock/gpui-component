## Plan
Replace panic/ignored-result paths in webview, LSP provider handling, reqwest_client proxy setup, and story theme persistence with explicit errors and diagnostics.

## Acceptance
- Webview setup/navigation failures do not panic silently.
- LSP provider failures no longer collapse into silent empty results.
- Invalid proxy config returns an error.
- Theme persistence distinguishes missing file from parse/write errors.