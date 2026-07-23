<p align="right">
  <a href="./README.en.md"><img alt="English" src="https://img.shields.io/badge/English-222?style=flat-square"></a>
  <a href="./README.md"><img alt="简体中文" src="https://img.shields.io/badge/简体中文-eee?style=flat-square"></a>
</p>

# xsearch

![xsearch turns one search query into structured sources](./assets/xsearch-banner.png)

**Search through the Grok proxy you already have.**

`xsearch` gives agents structured web search through any compatible third-party Grok endpoint. No MCP, no complicated setup, and no lock-in to a specific agent.

## Install

Install the skill definition and choose your agent and scope:

```bash
npx skills add catoncat/xsearch
```

No second step is required for normal use: on first use, the skill downloads the release for your platform and verifies its SHA-256 checksum.

To install and configure xsearch before first use, run the platform installer manually. It refreshes the skill and CLI in the default global install directory and creates a config template when one does not exist.

macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/catoncat/xsearch/main/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/catoncat/xsearch/main/install.ps1 | iex
```

## Configure

The CLI installer creates a local config file automatically and skips it if one already exists:

```text
macOS / Linux   ~/.config/xsearch/config.toml
Windows         %APPDATA%\xsearch\config.toml
```

After installation, edit the config file and add your proxy endpoint and model:

```toml
api_url = "https://your-grok-proxy.example/v1"
model = "your-grok-model"
```

Keep the key out of the file when possible:

```bash
export XSEARCH_API_KEY="your-provider-key"
```

Then ask your agent to search with xsearch. On hosts with slash skills, `/skill:xsearch your question` loads it explicitly; elsewhere, name xsearch in the request. The skill handles query planning, concurrent retrieval, source collection, and synthesis.

## Why it stays small

Search results are written to local artifacts. The agent receives a tiny receipt, reads the manifest, and loads only the result files it needs. Full evidence stays available without flooding the conversation context.

```text
query -> receipt -> manifest -> selected results
                         \-> complete report on disk
```

## Deep-read one page

When a conclusion rests on a specific page, the CLI ships an `extract` subcommand that fetches and reads it as Markdown — no API configuration needed:

```bash
xsearch extract "https://example.com/post"                    # complete content
xsearch extract "https://example.com/post" --format snippet   # first 500 chars
```

<details>
<summary><strong>CLI-only install</strong></summary>

macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/catoncat/xsearch/main/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/catoncat/xsearch/main/install.ps1 | iex
```

</details>

<details>
<summary><strong>Supported platforms</strong></summary>

- macOS: Apple Silicon and Intel
- Linux: ARM64 and x86_64
- Windows: x86_64

Release binaries and checksums are available on the [Releases page](https://github.com/catoncat/xsearch/releases).

</details>

<details>
<summary><strong>Build from source</strong></summary>

```bash
git clone https://github.com/catoncat/xsearch.git
cd xsearch
./scripts/install.sh
```

```bash
cd engine
cargo test --locked
cargo check --locked
```

</details>

## License

[MIT](./LICENSE)
