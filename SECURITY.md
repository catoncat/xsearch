# Security

## Reporting

Report security issues privately through GitHub Security Advisories for this repository. Do not open a public issue containing credentials or private endpoint details.

## Credential handling

`xsearch` reads endpoint configuration from `~/.config/xsearch/config.toml` or `XSEARCH_*` environment variables. Environment variables override file values.

- Prefer `XSEARCH_API_KEY` for credentials.
- If a key is stored in the config file, set its permissions to `0600`.
- Never commit a real config file, endpoint credential, or runtime log.
- Rotate a credential immediately if it is exposed in Git history or an issue.

The example configuration contains placeholders only.
