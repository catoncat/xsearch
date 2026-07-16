<p align="right">
  <a href="./README.en.md"><img alt="English" src="https://img.shields.io/badge/English-eee?style=flat-square"></a>
  <a href="./README.md"><img alt="简体中文" src="https://img.shields.io/badge/简体中文-222?style=flat-square"></a>
</p>

# xsearch

![xsearch：将一个搜索问题转换为结构化来源](./assets/xsearch-banner.png)

**把你已经在用的 Grok 反代，变成 Agent 的搜索工具。**

`xsearch` 通过兼容的第三方 Grok 模型接口，为 Agent 提供结构化网页搜索。不需要 MCP，不需要复杂的安装过程，任何支持 Skill 和命令执行的 Agent 都可以使用。

## 安装

第一步，安装 skill 定义并选择你的 Agent 和安装范围：

```bash
npx skills add catoncat/xsearch
```

第二步，安装 xsearch CLI。安装器会下载当前平台对应的发行版并校验 SHA-256。

macOS / Linux：

```bash
curl -fsSL https://raw.githubusercontent.com/catoncat/xsearch/main/install.sh | bash
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/catoncat/xsearch/main/install.ps1 | iex
```

## 配置

CLI 安装器会自动创建本地配置文件；如果已存在则跳过：

```text
macOS / Linux   ~/.config/xsearch/config.toml
Windows         %APPDATA%\xsearch\config.toml
```

安装完成后，编辑该配置文件，填入反代地址和模型名：

```toml
api_url = "https://your-grok-proxy.example/v1"
model = "your-grok-model"
```

API key 尽量放在环境变量里：

```bash
export XSEARCH_API_KEY="your-provider-key"
```

然后让 Agent 使用 xsearch 搜索。支持 slash skill 的宿主可用 `/skill:xsearch 你的问题` 明确调用；其他宿主直接点名 xsearch 即可。问题拆分、并发检索、来源整理和结果综合都由 skill 完成。

## 为什么不浪费上下文

完整搜索结果保存在本地 artifact 中。Agent 先收到一个很小的回执，再读取 manifest，只加载真正需要的结果文件。证据不会被截断，也不会一次性塞满对话上下文。

```text
问题 -> 回执 -> manifest -> 按需读取结果
                         \-> 完整报告保存在本地
```

<details>
<summary><strong>只安装 CLI</strong></summary>

macOS / Linux：

```bash
curl -fsSL https://raw.githubusercontent.com/catoncat/xsearch/main/install.sh | bash
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/catoncat/xsearch/main/install.ps1 | iex
```

</details>

<details>
<summary><strong>支持的平台</strong></summary>

- macOS：Apple Silicon、Intel
- Linux：ARM64、x86_64
- Windows：x86_64

预编译文件和校验和位于 [Releases](https://github.com/catoncat/xsearch/releases)。

</details>

<details>
<summary><strong>从源码构建</strong></summary>

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

## 许可证

[MIT](./LICENSE)
