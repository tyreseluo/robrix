# 部署指南：Robrix + Palpo + Octos

[English Version](deployment-guide.md)

本指南帮助你部署一套完整的 **Matrix AI 聊天系统**：Matrix 主服务器、AI 机器人后端，以及 Robrix 客户端——三者协同工作，让你在 Robrix 中与 AI 机器人对话。

> **只想快速试试？** 跳到 [快速开始](#2-快速开始) — 5 步即可运行。

---

## 目录

1. [这些项目是什么？](#1-这些项目是什么)
2. [快速开始](#2-快速开始)
3. [配置详解](#3-配置详解)
4. [使用 Robrix](#4-使用-robrix)
5. [端到端验证](#5-端到端验证)
6. [故障排除](#6-故障排除)
7. [延伸阅读](#7-延伸阅读)

---

## 1. 这些项目是什么？

三个开源项目协同工作，构成一个完整的 AI 聊天系统：

| 项目                                                             | 角色                  | 说明                                                                                                                                                                                                                   |
| ---------------------------------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [**Robrix**](https://github.com/Project-Robius-China/robrix2) | Matrix 客户端         | 用 Rust 编写的跨平台 Matrix 聊天客户端，基于[Makepad](https://github.com/makepad/makepad/) UI 框架。这是你看到并直接使用的程序——原生运行在 macOS、Linux、Windows、Android 和 iOS 上。                                   |
| [**Palpo**](https://github.com/palpo-im/palpo)                | Matrix 主服务器       | Rust 原生的 Matrix 主服务器。它存储用户、房间和消息，并在客户端（Robrix）和应用服务（Octos）之间路由事件。可以把它理解为整个系统的"邮局"。                                                                             |
| [**Octos**](https://github.com/octos-org/octos)               | AI 机器人（应用服务） | Rust 原生的 AI 智能体平台，以[Matrix Application Service](https://spec.matrix.org/latest/application-service-api/)（应用服务）身份运行。它从 Palpo 接收消息，发送给 LLM（如 DeepSeek、OpenAI 等），然后将 AI 的回复发回。 |

### 架构

```
┌──────────┐                        ┌──────────┐                       ┌──────────┐         ┌─────┐
│  Robrix  │  Client-Server API     │  Palpo   │  Appservice API       │  Octos   │  HTTPS  │ LLM │
│  (客户端) │ ────────────────────►  │  (服务器) │ ─────────────────►    │  (机器人) │ ──────► │     │
│          │ ◄──────────────────── │          │ ◄───────────────────  │          │ ◄────── │     │
└──────────┘   Sliding Sync        └──────────┘  Client-Server API    └──────────┘         └─────┘
  你的电脑                           Docker :8128     Docker :8009                          外部服务
```

**发送消息时的数据流：**

1. 你在 Robrix 中输入一条消息
2. Robrix 通过 Matrix Client-Server API 将消息发送给 Palpo
3. Palpo 发现该消息所在的房间有 Octos 存在，通过 Appservice API 将事件推送给 Octos
4. Octos 收到事件后，调用配置的 LLM（如 DeepSeek）获取回复
5. Octos 通过 Palpo 的 Client-Server API 将 AI 回复发回
6. Palpo 将回复推送给 Robrix，你看到机器人的回复

### 端口与协议

| 连接            | 协议                             | 默认端口                      | 备注                      |
| --------------- | -------------------------------- | ----------------------------- | ------------------------- |
| Robrix → Palpo | Client-Server API (Sliding Sync) | 8128（宿主机）→ 8008（容器） | Robrix 唯一需要访问的端口 |
| Palpo → Octos  | Appservice API                   | 8009（Docker 内部网络）       | Palpo 向 Octos 推送事件   |
| Octos → Palpo  | Client-Server API                | 8008（Docker 内部网络）       | Octos 通过 Palpo 回复消息 |
| Octos 控制面板  | HTTP                             | 8010（宿主机）→ 8080（容器） | 可选的管理界面            |
| Octos → LLM    | HTTPS                            | 443（出站）                   | 外部 API 调用             |

---

## 2. 快速开始

4 步在本地跑通所有服务。

### 前提条件

- **Docker** 和 **Docker Compose**（v2+）
- **Git**
- **一个 LLM API Key** — 如 [DeepSeek](https://platform.deepseek.com/)（有免费额度）
- **Robrix** — [下载预编译版本](https://github.com/Project-Robius-China/robrix2/releases)，或从源码构建：`cargo run --release`

### 步骤 1：获取示例配置

```bash
git clone https://github.com/Project-Robius-China/robrix2.git
cd robrix2/docs/examples
```

### 步骤 2：运行初始化脚本

```bash
./setup.sh
```

此脚本会克隆 Palpo 和 Octos 的源码仓库，并创建 `.env` 文件。两者均从源码构建，以支持所有架构（x86_64、ARM64/Apple Silicon 等）。

### 步骤 3：设置 API Key

编辑 `.env`，将 `your-api-key-here` 替换为你的 DeepSeek API Key：

```
DEEPSEEK_API_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxxx
```

### 步骤 4：启动服务

```bash
docker compose up -d
```

> **注意：** 首次运行会从源码编译 Palpo 和 Octos，这可能需要 **10–30 分钟**（取决于你的机器性能和网络速度）。Palpo 需要编译其 Rust 代码；Octos 还额外需要下载 Node.js、Chromium 等技能插件的运行时工具。后续启动会使用缓存镜像，几秒内完成。

检查运行状态：

```bash
docker compose ps
```

你应该看到三个服务（`palpo_postgres`、`palpo`、`octos`）都处于 `running` 状态。

### 步骤 5：用 Robrix 连接（构建完成后）

1. **打开 Robrix**（还没有？见 [4.1 获取 Robrix](#41-获取-robrix)）
2. **设置服务器地址**：在登录界面，在 **Homeserver URL** 输入框中（密码框下方）输入 `http://127.0.0.1:8128`

   <!-- screenshot: login-screen.png — Robrix 登录界面，高亮 homeserver 输入框 -->
3. **注册新账号**：输入用户名和密码，点击 **Sign up**

   ![注册账号 — 输入用户名、密码和服务器地址](images/register-account.png)
4. **与 AI 机器人对话**：登录后，加入或创建一个房间，然后邀请机器人：

   - 点击房间中的邀请按钮
   - 输入 `@octosbot:127.0.0.1:8128`
   - 发送一条消息——AI 机器人应该会回复！

   <!-- screenshot: bot-chat.png — 与 AI 机器人的对话 -->

**完成！** 你现在拥有了一个可工作的 Robrix + Palpo + Octos 系统。继续阅读了解配置详情，或跳到 [故障排除](#6-故障排除) 解决问题。

---

## 3. 配置详解

本节解释 `examples/` 目录中的每个配置文件。快速开始已经让你跑起来了——当你需要自定义时再来这里查阅。

### 3.1 目录结构

```
examples/
├── compose.yml                         # Docker Compose — 编排所有服务
├── .env.example                        # 环境变量模板
├── palpo.toml                          # Palpo 主服务器配置
├── appservices/
│   └── octos-registration.yaml         # 应用服务注册文件（连接 Palpo ↔ Octos）
├── config/
│   ├── botfather.json                  # Octos 机器人配置（Matrix 通道）
│   └── octos.json                      # Octos 全局设置
├── data/                               # 持久化数据（运行时自动创建）
│   ├── pgsql/                          # PostgreSQL 数据库文件
│   ├── octos/                          # Octos 运行时数据
│   └── media/                          # Palpo 媒体存储
└── static/
    └── index.html                      # Palpo 首页（可选）
```

### 3.2 令牌生成

应用服务注册文件和 Octos 机器人配置共享两个密钥令牌，用于双向认证。示例文件中已预填开发用令牌，但**在生产环境中你必须重新生成**：

```bash
openssl rand -hex 32   # → 用作 as_token
openssl rand -hex 32   # → 用作 hs_token
```

这两个值必须在 `appservices/octos-registration.yaml` 和 `config/botfather.json` 中完全一致。如果不匹配，机器人将无法工作。详见 [令牌匹配检查清单](#37-令牌匹配检查清单)。

### 3.3 应用服务注册文件（`appservices/octos-registration.yaml`）

此文件告诉 Palpo 关于 Octos 的信息——Octos 管理哪些用户命名空间，以及将事件发送到哪里。

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"

as_token: "<你的-as-token>"
hs_token: "<你的-hs-token>"

sender_localpart: octosbot
rate_limited: false

namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:127\\.0\\.0\\.1:8128"
    - exclusive: true
      regex: "@octosbot:127\\.0\\.0\\.1:8128"
  aliases: []
  rooms: []
```

| 字段                 | 说明                                                                                                                   |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `id`               | 此应用服务注册的唯一标识符。                                                                                           |
| `url`              | Palpo 发送事件的目标地址。使用 Docker 服务名 `octos`（不是 `localhost`），因为两个容器在同一个 Docker 网络中。     |
| `as_token`         | Octos 调用 Palpo API 时使用的令牌。必须与 `botfather.json` 匹配。                                                    |
| `hs_token`         | Palpo 向 Octos 推送事件时使用的令牌。必须与 `botfather.json` 匹配。                                                  |
| `sender_localpart` | 机器人的 Matrix 本地用户名。最终变为 `@octosbot:127.0.0.1:8128`。                                                  |
| `rate_limited`     | 设为 `false`，让机器人回复不受速率限制。                                                                             |
| `namespaces.users` | 此应用服务管理的用户 ID 正则匹配模式。包含机器人本身（`@octosbot:...`）和动态创建的子机器人（`@octosbot_*:...`）。 |

### 3.4 Palpo 配置（`palpo.toml`）

```toml
server_name = "127.0.0.1:8128"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true

appservice_registration_dir = "/var/palpo/appservices"

# HTTP 监听器（Client-Server API）
[[listeners]]
address = "0.0.0.0:8008"

[logger]
format = "pretty"

[db]
url = "postgres://palpo:palpo_dev_password@palpo_postgres:5432/palpo"
pool_size = 10

[well_known]
server = "127.0.0.1:8128"
client = "http://127.0.0.1:8128"
```

| 字段                            | 说明                                                                                                                   |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `server_name`                 | 所有 Matrix ID 的域名部分（如 `@user:127.0.0.1:8128`）。生产环境使用你的实际域名。                                  |
| `allow_registration`          | 是否允许新用户注册。设为 `true` 以便 Robrix 用户可以创建账号。生产环境中初始配置完成后可改为 `false`。             |
| `yes_i_am_very_very_sure_...` | 当 `allow_registration = true` 时必填的安全确认。字段名故意很长，提醒你开放注册的安全风险。                          |
| `enable_admin_room`           | 启用服务器管理员房间。                                                                                                 |
| `appservice_registration_dir` | Palpo 启动时自动加载此目录下所有 `.yaml` 文件。Octos 就是通过这种方式被发现的。                                      |
| `[[listeners]]`               | 网络监听器。每个条目定义一个 Palpo 监听的地址。                                                                        |
| `[logger]`                    | 日志格式。`"pretty"` 用于开发，`"json"` 用于生产。                                                                 |
| `[db]`                        | PostgreSQL 连接配置。`palpo_postgres` 是 Docker 服务名。密码必须与 `compose.yml` 中的 `POSTGRES_PASSWORD` 匹配。 |
| `[well_known]`                | 用于客户端发现服务器。必须与外部可访问的地址匹配。                                                                     |

> **注意：** 在这个本地 Docker 示例里，Matrix 身份统一使用 `127.0.0.1:8128`。因此 `server_name`、应用服务正则和机器人用户 ID 都必须写成 `127.0.0.1:8128`。只有容器之间通信时才使用 `palpo:8008`、`octos:8009` 这类 Docker 服务名。

> **延伸阅读：** [Palpo GitHub](https://github.com/palpo-im/palpo) 了解更多高级配置（联邦、TLS、TURN 等）。

### 3.5 Octos 机器人配置（`config/botfather.json`）

此文件定义机器人的身份、LLM 提供商和 Matrix 通道配置。

```json
{
  "id": "botfather",
  "name": "BotFather",
  "enabled": true,
  "config": {
    "provider": "deepseek",
    "model": "deepseek-chat",
    "api_key_env": "DEEPSEEK_API_KEY",
    "channels": [
      {
        "type": "matrix",
        "homeserver": "http://palpo:8008",
        "as_token": "<你的-as-token>",
        "hs_token": "<你的-hs-token>",
        "server_name": "127.0.0.1:8128",
        "sender_localpart": "octosbot",
        "user_prefix": "octosbot_",
        "port": 8009,
        "allowed_senders": []
      }
    ],
    "gateway": {
      "max_history": 50,
      "queue_mode": "followup"
    }
  },
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z"
}
```

> **重要：** `created_at` 和 `updated_at` 字段是 Octos **必需的**。如果缺少这两个字段，Octos 会跳过该 profile，机器人将无法启动。

**LLM 提供商设置：**

| 字段            | 说明                                                                                                                   |
| --------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `provider`    | LLM 提供商名称。Octos 支持 `deepseek`、`openai`、`anthropic` 等 [14 种提供商](https://octos-org.github.io/octos/)。 |
| `model`       | 模型标识符（如 `deepseek-chat`、`gpt-4o`、`claude-sonnet-4-20250514`）。                                         |
| `api_key_env` | 存放 API Key 的环境变量名称。                                                                                          |

**Matrix 通道设置：**

| 字段                        | 说明                                                                  |
| --------------------------- | --------------------------------------------------------------------- |
| `type`                    | 必须为 `"matrix"`。                                                 |
| `homeserver`              | Palpo 的内部 URL。使用 Docker 服务名 `palpo`，不是 `localhost`。  |
| `as_token` / `hs_token` | 必须与应用服务注册 YAML 文件匹配。                                    |
| `server_name`             | Matrix 域名。必须与 `palpo.toml` 中的 `server_name` 一致。        |
| `sender_localpart`        | 机器人用户名。必须与注册文件一致。                                    |
| `user_prefix`             | 动态创建的子机器人用户 ID 前缀（如 `octosbot_translator`）。        |
| `port`                    | Octos 监听 Palpo 应用服务事件的端口。                                 |
| `allowed_senders`         | 允许与机器人对话的 Matrix 用户 ID。空数组 `[]` = 所有人都可以对话。 |

> **注意：** `homeserver` 是 Octos 访问 Palpo 时使用的 Docker 内部 URL；`server_name` 是写进 Matrix 用户 ID 的域名部分。两者相关，但不能混用。

**Gateway 设置：**

| 字段            | 说明                                                            |
| --------------- | --------------------------------------------------------------- |
| `max_history` | 作为 LLM 上下文发送的最大历史消息数量。                         |
| `queue_mode`  | Octos 处理传入消息的方式。`followup` 将新消息排队并顺序处理。 |

> **延伸阅读：** [Octos Book — LLM 提供商与路由](https://octos-org.github.io/octos/) 了解全部 14 种提供商、降级链和自适应路由。

### 3.6 Docker Compose（`compose.yml`）

提供的 `compose.yml` 启动三个服务：

| 服务               | 镜像                              | 暴露端口                     | 用途              |
| ------------------ | --------------------------------- | ---------------------------- | ----------------- |
| `palpo_postgres` | `postgres:17`                   | *（无，仅内部）*           | Palpo 的数据库    |
| `palpo`          | 从源码构建 | `8128:8008`                | Matrix 主服务器   |
| `octos`          | 从源码构建                        | `8009:8009`、`8010:8080` | AI 机器人应用服务 |

**端口映射说明：**

- `8128` → Robrix 连接此端口（Client-Server API）
- `8009` → Palpo 向 Octos 推送事件（Appservice API）
- `8010` → Octos 管理控制面板（可选，用于监控）

**持久化卷：**

| 卷               | 用途                                              |
| ---------------- | ------------------------------------------------- |
| `./data/pgsql` | PostgreSQL 数据。`docker compose down` 后保留。 |
| `./data/octos` | Octos 运行时数据（会话、记忆）。                  |
| `./data/media` | 通过 Matrix 上传的媒体文件（图片、文件）。        |

**环境变量（`.env`）：**

| 变量                 | 必填         | 默认值                 | 说明                    |
| -------------------- | ------------ | ---------------------- | ----------------------- |
| `DEEPSEEK_API_KEY` | **是** | —                     | 你的 LLM API Key        |
| `DB_PASSWORD`      | 否           | `palpo_dev_password` | PostgreSQL 密码         |
| `RUST_LOG`         | 否           | `octos=debug,info`   | 日志详细程度            |

### 3.7 令牌匹配检查清单

最常见的配置错误是令牌不匹配。以下值在两个文件中**必须完全一致**：

| 值                   | 在 `octos-registration.yaml` 中 | 在 `botfather.json` 中           |
| -------------------- | --------------------------------- | ---------------------------------- |
| `as_token`         | `as_token: "abc..."`            | `"as_token": "abc..."`           |
| `hs_token`         | `hs_token: "def..."`            | `"hs_token": "def..."`           |
| `sender_localpart` | `sender_localpart: octosbot`    | `"sender_localpart": "octosbot"` |
| `server_name`      | `regex: "@octosbot:127\\.0\\.0\\.1:8128"` | `"server_name": "127.0.0.1:8128"` |

如果有任何不匹配，机器人将不会响应消息。提交 bug 报告前请先检查！

---

## 4. 使用 Robrix

本节介绍如何使用 Robrix 客户端连接 Palpo 服务器并与 Octos AI 机器人交互。

### 4.1 获取 Robrix

**下载预编译版本（推荐）：**

从 [Robrix 发布页面](https://github.com/Project-Robius-China/robrix2/releases) 下载。支持 macOS、Linux 和 Windows。

**或从源码构建：**

1. [安装 Rust](https://www.rust-lang.org/tools/install)
2. Linux 上安装依赖：
   ```bash
   sudo apt-get install libssl-dev libsqlite3-dev pkg-config libxcursor-dev libx11-dev libasound2-dev libpulse-dev libwayland-dev libxkbcommon-dev
   ```
3. 构建并运行：
   ```bash
   cargo run --release
   ```

移动端构建（Android/iOS）和打包分发的说明，详见 [Robrix README](https://github.com/Project-Robius-China/robrix2#building--running-robrix-on-desktop)。

### 4.2 连接到 Palpo 服务器

启动 Robrix 后，你会看到登录界面：

<!-- screenshot: login-screen.png — 完整的登录界面 -->

**Homeserver URL** 输入框位于登录表单底部。如果留空，默认连接 `matrix.org`。要连接你的本地 Palpo 实例：

- **本地部署：** 输入 `http://127.0.0.1:8128`
- **远程部署：** 输入 `https://your.server.name`（或 `http://服务器IP:8128`）

<!-- screenshot: homeserver-input.png — 高亮 Homeserver URL 输入框 -->

> **注意：** Robrix 要求主服务器支持 [Sliding Sync](https://spec.matrix.org/latest/client-server-api/#sliding-sync)。Palpo 原生支持此功能。

### 4.3 注册与登录

**首次使用——注册新账号：**

1. 输入你想要的**用户名**和**密码**
2. 在**确认密码**栏（注册时出现）再次输入密码
3. 输入 **Homeserver URL**（如 `http://127.0.0.1:8128`）
4. 点击 **Sign up**

![注册账号 — 输入用户名、密码和服务器地址](images/register-account.png)

**再次使用——登录：**

1. 输入**用户名**和**密码**
2. 输入 **Homeserver URL**
3. 点击 **Log in**

登录成功后，你会看到房间列表（新账号为空）。

<!-- screenshot: room-list.png — 登录后的房间列表 -->

### 4.4 与 AI 机器人交互

有两种方式开始与机器人聊天：

#### 方式一：邀请机器人到房间

1. 创建一个新房间或打开现有房间
2. 点击房间中的**邀请**按钮
3. 输入机器人的 Matrix ID：`@octosbot:127.0.0.1:8128`（将 `127.0.0.1:8128` 替换为你的 `server_name`）
4. 机器人会自动加入房间

<!-- screenshot: invite-bot.png — 邀请弹窗中输入了 @octosbot:127.0.0.1:8128 -->

#### 方式二：加入机器人所在的房间

1. 点击**加入房间**按钮（或使用房间浏览器）
2. 输入机器人已设置的房间别名或 ID
3. 开始聊天

<!-- screenshot: join-room.png — 加入房间对话框 -->

#### 与机器人对话

机器人加入房间后，直接输入消息并发送即可。机器人会通过配置的 LLM 处理你的消息并回复。

<!-- screenshot: bot-chat.png — 展示用户消息和 AI 机器人回复的对话 -->

### 4.5 机器人管理（高级功能）

Robrix 内置了通过 BotFather 系统管理 Matrix 机器人的功能。

#### 启用应用服务支持

1. 打开 Robrix 的**设置**
2. 导航到 **Bot Settings**
3. 开启 **Enable App Service**
4. 输入 **BotFather User ID**（如 `@octosbot:127.0.0.1:8128`）
5. 点击 **Save**

<!-- screenshot: bot-settings.png — Bot Settings 界面 -->

#### 创建子机器人

启用 BotFather 后，你可以创建专用的子机器人：

1. 使用 **Create Bot** 对话框
2. 填写：
   - **Username** — 仅限小写字母、数字和下划线（如 `translator_bot`）
   - **Display Name** — 可读的显示名称（如 "翻译机器人"）
   - **System Prompt** — 机器人的初始指令（如 "你是一个翻译器。将所有消息翻译成中文。"）
3. 点击 **Create Bot**

机器人将以 `@octosbot_<username>:127.0.0.1:8128` 的身份创建。

<!-- screenshot: create-bot.png — Create Bot 弹窗 -->

---

## 5. 端到端验证

部署完成后，按照以下检查清单确认一切正常：

### 服务健康检查

```bash
# 检查所有容器是否运行
docker compose ps

# 检查 Palpo 日志是否有启动错误
docker compose logs palpo | tail -20

# 检查 Octos 日志——寻找 "appservice listening" 或类似信息
docker compose logs octos | tail -20

# 验证 Palpo 是否响应
curl -s http://127.0.0.1:8128/_matrix/client/versions | head -5
```

### 客户端连接

- [ ] Robrix 能连接到 `http://127.0.0.1:8128`
- [ ] 能注册新账号
- [ ] 登录后房间列表能加载（新账号可能为空）
- [ ] 能创建新房间

### 机器人交互

- [ ] 能邀请 `@octosbot:127.0.0.1:8128` 到房间
- [ ] 机器人加入房间（如果没有，检查 `docker compose logs octos`）
- [ ] 发送消息后机器人回复
- [ ] 回复内容合理（确认 LLM 连接正常）

### 如果某步失败

按照数据流顺序检查日志：

```bash
# 1. Palpo 是否收到了 Robrix 的消息？
docker compose logs palpo --since 1m

# 2. Palpo 是否将事件转发给了 Octos？
docker compose logs palpo --since 1m | grep -i appservice

# 3. Octos 是否收到并处理了事件？
docker compose logs octos --since 1m

# 4. Octos 是否成功调用了 LLM？
docker compose logs octos --since 1m | grep -i -E "deepseek|llm|provider"
```

---

## 6. 故障排除

### 6.1 服务启动问题

| 症状                        | 原因                                    | 解决方法                                                                                      |
| --------------------------- | --------------------------------------- | --------------------------------------------------------------------------------------------- |
| `palpo_postgres` 无法启动 | 端口 5432 已被占用，或数据损坏          | 检查 `docker compose logs palpo_postgres`。删除 `data/pgsql/` 重新开始。                  |
| `palpo` 构建失败           | 网络问题或源码获取失败                 | 确保 Docker 能访问 `github.com`。检查 `docker compose logs palpo` 查看构建错误。           |
| `palpo` 启动时崩溃        | `palpo.toml` 语法错误或数据库连接失败 | 检查日志。确保 `palpo_postgres` 先正常运行。验证数据库密码一致。                            |
| `octos` 构建失败          | 缺少 Dockerfile 或网络问题              | 确保 Docker 能访问 `github.com`。或者在本地构建 Octos 并修改 `compose.yml` 使用本地镜像。 |
| `octos` 启动但日志有错误  | `botfather.json` 无效或缺少 API Key   | 检查 JSON 语法。验证 `.env` 中已设置 `DEEPSEEK_API_KEY`。                                 |

### 6.2 Robrix 连接问题

| 症状                             | 原因                                                                    | 解决方法                                                                              |
| -------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| "无法连接到服务器"               | Homeserver URL 错误或 Palpo 未运行                                      | 确认 Palpo 正在运行（`docker compose ps`）。确认 URL 为 `http://127.0.0.1:8128`。 |
| 登录成功但没有房间               | 新账号的正常现象                                                        | 创建一个新房间。加入或创建后房间会出现在列表中。                                      |
| 注册失败                         | `palpo.toml` 中 `allow_registration = false`，或 server_name 不匹配 | 检查 `palpo.toml`。确保 `allow_registration = true`。                             |
| "Homeserver 不支持 Sliding Sync" | Palpo 版本过旧                                                          | 重新构建 Palpo：`docker compose build --no-cache palpo`。                              |
| 连接超时                         | 防火墙阻止了端口 8128                                                   | 检查防火墙规则。macOS 上在系统设置中允许传入连接。                                    |

### 6.3 机器人问题

| 症状                                    | 原因                                                    | 解决方法                                                                                             |
| --------------------------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 机器人不响应消息                        | 注册文件和配置文件之间令牌不匹配                        | 验证[令牌匹配检查清单](#37-令牌匹配检查清单)。                                                          |
| Palpo 日志中出现 `Connection refused` | Octos 未运行，或注册 YAML 中 `url` 错误               | 确保 Octos 正在运行。`url` 必须使用 Docker 服务名（`http://octos:8009`），不能用 `localhost`。 |
| `User ID not in namespace`            | `sender_localpart` 与 `namespaces.users` 正则不匹配 | 更新 `octos-registration.yaml` 中的正则表达式，包含机器人的完整用户 ID 模式。                      |
| 机器人加入房间但回复空消息              | LLM API Key 无效或额度不足                              | 检查 `docker compose logs octos` 中的 API 错误。验证 API Key 和账户余额。                          |
| 部分用户的消息被忽略                    | `botfather.json` 中的 `allowed_senders` 过滤        | 将用户的 Matrix ID 添加到 `allowed_senders` 数组中，或设为 `[]` 允许所有人。                     |

### 6.4 常用调试命令

```bash
# 实时查看所有服务日志
docker compose logs -f

# 查看特定服务的日志
docker compose logs -f palpo
docker compose logs -f octos

# 重启单个服务
docker compose restart octos

# 检查 Palpo 的 API
curl http://127.0.0.1:8128/_matrix/client/versions

# 完全重置（警告：删除所有数据）
docker compose down -v
rm -rf data/
docker compose up -d
```

---

## 7. 延伸阅读

- **Octos 完整文档：** [octos-org.github.io/octos](https://octos-org.github.io/octos/) — 覆盖所有 LLM 提供商、通道（Telegram、Slack、Discord 等）、技能、记忆系统和高级配置。
- **Octos Matrix Appservice 指南：** [octos-org/octos#171](https://github.com/octos-org/octos/pull/171) — 本文档参考的原始指南，包含更多上下文。
- **Palpo：** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) — Palpo 主服务器文档。
- **Robrix：** [Project-Robius-China/robrix2](https://github.com/Project-Robius-China/robrix2) — Robrix 客户端、构建说明和功能追踪。
- **Matrix Appservice 规范：** [spec.matrix.org — Application Service API](https://spec.matrix.org/latest/application-service-api/) — 应用服务的 Matrix 协议规范。

---

*本指南内容截至 2026 年 4 月。最新更新请查看各项目的仓库。*
