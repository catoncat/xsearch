<p align="right">
  <a href="./README.en.md"><img alt="English" src="https://img.shields.io/badge/English-222?style=flat-square"></a>
  <a href="./README.md"><img alt="简体中文" src="https://img.shields.io/badge/简体中文-eee?style=flat-square"></a>
</p>

# xsearch

![xsearch turns one search query into structured sources](./assets/xsearch-banner.png)

**Search through the Grok proxy you already have.**

`xsearch` gives agents structured web search through any compatible third-party Grok endpoint. No MCP, no complicated setup, and no lock-in to a specific agent.

## Install

```bash
npx skills add catoncat/xsearch
```

Pick your agent and scope. On first use, the skill downloads the correct signed release binary for your platform and verifies its SHA-256 checksum.

## Configure

The first run creates a local config file:

```text
macOS / Linux   ~/.config/xsearch/config.toml
Windows         %APPDATA%\xsearch\config.toml
```

Add your proxy endpoint and model:

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
