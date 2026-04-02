# Deploying Robrix with Palpo and Octos

[中文版 (Chinese Version)](deployment-guide-zh.md)

This guide walks you through deploying a complete **Matrix AI chat system**: a Matrix homeserver, an AI bot backend, and the Robrix client — all working together so you can chat with an AI bot from Robrix.

> **Just want to try it quickly?** Jump to [Quick Start](#2-quick-start) — 5 steps to get running.

---

## Table of Contents

1. [What Are These Projects?](#1-what-are-these-projects)
2. [Quick Start](#2-quick-start)
3. [Configuration Details](#3-configuration-details)
4. [Using Robrix](#4-using-robrix)
5. [End-to-End Verification](#5-end-to-end-verification)
6. [Troubleshooting](#6-troubleshooting)
7. [Further Reading](#7-further-reading)

---

## 1. What Are These Projects?

Three open-source projects work together to form a complete AI chat system:

| Project | Role | What it does |
|---------|------|-------------|
| [**Robrix**](https://github.com/Project-Robius-China/robrix2) | Matrix Client | A cross-platform Matrix chat client written in Rust using [Makepad](https://github.com/makepad/makepad/). This is what you see and interact with — it runs natively on macOS, Linux, Windows, Android, and iOS. |
| [**Palpo**](https://github.com/palpo-im/palpo) | Matrix Homeserver | A Rust-native Matrix homeserver. It stores users, rooms, and messages, and routes events between clients (Robrix) and application services (Octos). Think of it as the "post office" of the system. |
| [**Octos**](https://github.com/octos-org/octos) | AI Bot (Appservice) | A Rust-native AI agent platform that runs as a [Matrix Application Service](https://spec.matrix.org/latest/application-service-api/). It receives messages from Palpo, sends them to an LLM (like DeepSeek, OpenAI, etc.), and posts the AI's reply back. |

### Architecture

```
┌──────────┐                        ┌──────────┐                       ┌──────────┐         ┌─────┐
│  Robrix  │  Client-Server API     │  Palpo   │  Appservice API       │  Octos   │  HTTPS  │ LLM │
│ (Client) │ ────────────────────►  │ (Server) │ ─────────────────►    │  (Bot)   │ ──────► │     │
│          │ ◄──────────────────── │          │ ◄───────────────────  │          │ ◄────── │     │
└──────────┘   Sliding Sync        └──────────┘  Client-Server API    └──────────┘         └─────┘
  Your machine                     Docker :8128      Docker :8009                          External
```

**Data flow when you send a message:**

1. You type a message in Robrix
2. Robrix sends it to Palpo via the Matrix Client-Server API
3. Palpo sees the message is in a room where Octos is present, and pushes the event to Octos via the Appservice API
4. Octos receives the event, calls the configured LLM (e.g., DeepSeek), and gets a response
5. Octos posts the AI reply back through Palpo's Client-Server API
6. Palpo delivers the reply to Robrix, where you see the bot's response

### Ports and Protocols

| Connection | Protocol | Default Port | Notes |
|-----------|----------|-------------|-------|
| Robrix → Palpo | Client-Server API (Sliding Sync) | 8128 (host) → 8008 (container) | The only port Robrix needs |
| Palpo → Octos | Appservice API | 8009 (internal Docker network) | Palpo pushes events to Octos |
| Octos → Palpo | Client-Server API | 8008 (internal Docker network) | Octos replies through Palpo |
| Octos Dashboard | HTTP | 8010 (host) → 8080 (container) | Optional admin UI |
| Octos → LLM | HTTPS | 443 (outbound) | External API call |

---

## 2. Quick Start

Get everything running locally in 4 steps.

### Prerequisites

- **Docker** and **Docker Compose** (v2+)
- **Git**
- **An LLM API key** — e.g., [DeepSeek](https://platform.deepseek.com/) (free tier available)
- **Robrix** — [download a pre-built release](https://github.com/Project-Robius-China/robrix2/releases), or build from source with `cargo run --release`

### Step 1: Get the Example Configuration

```bash
git clone https://github.com/Project-Robius-China/robrix2.git
cd robrix2/docs/examples
```

### Step 2: Run Setup

```bash
./setup.sh
```

This clones the Palpo and Octos source repos and creates your `.env` file. Both are built from source to support all architectures (x86_64, ARM64/Apple Silicon, etc.).

### Step 3: Set Your API Key

Edit `.env` and replace `your-api-key-here` with your actual DeepSeek API key:

```
DEEPSEEK_API_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxxx
```

### Step 4: Start the Services

```bash
docker compose up -d
```

> **Note:** The first run builds both Palpo and Octos from source, which can take **10–30 minutes** depending on your machine and network speed. Palpo compiles its Rust codebase; Octos additionally downloads runtime tools (Node.js, Chromium) for its skill plugins. Subsequent runs use cached images and start in seconds.

Check that everything is running:

```bash
docker compose ps
```

You should see three services (`palpo_postgres`, `palpo`, `octos`) all in `running` state.

### Step 5: Connect with Robrix (after build completes)

1. **Open Robrix** (don't have it yet? See [4.1 Getting Robrix](#41-getting-robrix))

2. **Set the homeserver**: In the login screen, enter `http://127.0.0.1:8128` in the **Homeserver URL** field (below the password field).

   <!-- screenshot: login-screen.png — Robrix login screen with homeserver URL field highlighted -->

3. **Register a new account**: Enter a username and password, then click **Sign up**.

   ![Register account — enter username, password, and homeserver URL](images/register-account.png)

4. **Talk to the AI bot**: After logging in, join a room or create one, then invite the bot:
   - Click the invite button in the room
   - Enter `@octosbot:127.0.0.1:8128`
   - Send a message — the AI bot should reply!

   <!-- screenshot: bot-chat.png — Conversation with the AI bot -->

**That's it!** You now have a working Robrix + Palpo + Octos setup. Read on for configuration details, or jump to [Troubleshooting](#6-troubleshooting) if something isn't working.

---

## 3. Configuration Details

This section explains every configuration file in the `examples/` directory. You already have a working setup from the Quick Start — come here when you want to customize.

### 3.1 Directory Layout

```
examples/
├── compose.yml                         # Docker Compose — orchestrates all services
├── .env.example                        # Environment variables template
├── palpo.toml                          # Palpo homeserver configuration
├── appservices/
│   └── octos-registration.yaml         # Appservice registration (links Palpo ↔ Octos)
├── config/
│   ├── botfather.json                  # Octos bot profile (Matrix channel config)
│   └── octos.json                      # Octos global settings
├── data/                               # Persistent data (created at runtime)
│   ├── pgsql/                          # PostgreSQL database files
│   ├── octos/                          # Octos runtime data
│   └── media/                          # Palpo media storage
└── static/
    └── index.html                      # Palpo homepage (optional)
```

### 3.2 Token Generation

The Appservice registration and the Octos bot profile share two secret tokens for mutual authentication. The example files come with pre-filled development tokens, but **you must generate new tokens for production**:

```bash
openssl rand -hex 32   # → use as as_token
openssl rand -hex 32   # → use as hs_token
```

These two values must be identical in `appservices/octos-registration.yaml` and `config/botfather.json`. If they don't match, nothing works. See [Token Matching Checklist](#37-token-matching-checklist).

### 3.3 Appservice Registration (`appservices/octos-registration.yaml`)

This file tells Palpo about Octos — which user namespaces Octos manages and where to send events.

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"

as_token: "<your-as-token>"
hs_token: "<your-hs-token>"

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

| Field | Description |
|-------|-------------|
| `id` | A unique identifier for this appservice registration. |
| `url` | Where Palpo sends events. Uses the Docker service name `octos` (not `localhost`), because both containers share the same Docker network. |
| `as_token` | Token that Octos uses when calling Palpo's API. Must match `botfather.json`. |
| `hs_token` | Token that Palpo uses when pushing events to Octos. Must match `botfather.json`. |
| `sender_localpart` | The bot's Matrix local username. Becomes `@octosbot:127.0.0.1:8128`. |
| `rate_limited` | Set to `false` so the bot can respond without rate limits. |
| `namespaces.users` | Regex patterns for user IDs that this appservice owns. Include the bot itself (`@octosbot:...`) and any dynamically-created bot users (`@octosbot_*:...`). |

### 3.4 Palpo Configuration (`palpo.toml`)

```toml
server_name = "127.0.0.1:8128"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true

appservice_registration_dir = "/var/palpo/appservices"

# HTTP listener (Client-Server API)
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

| Field | Description |
|-------|-------------|
| `server_name` | The domain part of all Matrix IDs (e.g., `@user:127.0.0.1:8128`). For production, use your actual domain. |
| `allow_registration` | Whether new users can register. Set to `true` so Robrix users can create accounts. For production, consider setting to `false` after initial setup. |
| `yes_i_am_very_very_sure_...` | Required safety confirmation when `allow_registration = true`. The intentionally long name reminds you of the security implications. |
| `enable_admin_room` | Enables the server admin room for management. |
| `appservice_registration_dir` | Palpo loads all `.yaml` files from this directory on startup. This is how it discovers Octos. |
| `[[listeners]]` | Network listeners. Each entry defines an address Palpo listens on. |
| `[logger]` | Log format. `"pretty"` for development, `"json"` for production. |
| `[db]` | PostgreSQL connection. `palpo_postgres` is the Docker service name. The password must match `POSTGRES_PASSWORD` in `compose.yml`. |
| `[well_known]` | Used by clients for server discovery. Must match externally-reachable addresses. |

> **Important:** In this local Docker example, the Matrix identity is `127.0.0.1:8128`, so `server_name`, the appservice regex, and bot user IDs must all use `127.0.0.1:8128`. Only container-to-container traffic should use Docker service names like `palpo:8008` or `octos:8009`.

> **Further reading:** [Palpo on GitHub](https://github.com/palpo-im/palpo) for advanced configuration (federation, TLS, TURN, etc.).

### 3.5 Octos Bot Profile (`config/botfather.json`)

This file defines the bot's identity, LLM provider, and Matrix channel configuration.

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
        "as_token": "<your-as-token>",
        "hs_token": "<your-hs-token>",
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

> **Important:** The `created_at` and `updated_at` fields are **required** by Octos. If they are missing, Octos will skip this profile and the bot will never start.

**LLM Provider settings:**

| Field | Description |
|-------|-------------|
| `provider` | LLM provider name. Octos supports `deepseek`, `openai`, `anthropic`, and [12 more](https://octos-org.github.io/octos/). |
| `model` | Model identifier (e.g., `deepseek-chat`, `gpt-4o`, `claude-sonnet-4-20250514`). |
| `api_key_env` | Name of the environment variable holding your API key. |

**Matrix channel settings:**

| Field | Description |
|-------|-------------|
| `type` | Must be `"matrix"`. |
| `homeserver` | Palpo's internal URL. Uses Docker service name `palpo`, not `localhost`. |
| `as_token` / `hs_token` | Must match the appservice registration YAML. |
| `server_name` | The Matrix domain. Must match `server_name` in `palpo.toml`. |
| `sender_localpart` | Bot username. Must match the registration file. |
| `user_prefix` | Prefix for dynamically-created bot users (e.g., `octosbot_translator`). |
| `port` | Port Octos listens on for Appservice events from Palpo. |
| `allowed_senders` | Matrix user IDs allowed to talk to the bot. Empty `[]` = everyone can talk to it. |

> **Important:** `homeserver` is the internal Docker URL Octos uses to call Palpo. `server_name` is the Matrix domain embedded in user IDs. They are related, but they are not interchangeable.

**Gateway settings:**

| Field | Description |
|-------|-------------|
| `max_history` | Maximum number of messages to include as context for the LLM. |
| `queue_mode` | How Octos handles incoming messages. `followup` queues new messages and processes them sequentially. |

> **Further reading:** [Octos Book — LLM Providers & Routing](https://octos-org.github.io/octos/) for all 14 supported providers, fallback chains, and adaptive routing.

### 3.6 Docker Compose (`compose.yml`)

The provided `compose.yml` starts three services:

| Service | Image | Exposed Ports | Purpose |
|---------|-------|--------------|---------|
| `palpo_postgres` | `postgres:17` | *(none, internal only)* | Database for Palpo |
| `palpo` | Built from source | `8128:8008` | Matrix homeserver |
| `octos` | Built from source | `8009:8009`, `8010:8080` | AI bot appservice |

**Port mapping explanation:**

- `8128` → Robrix connects here (Client-Server API)
- `8009` → Palpo pushes events to Octos here (Appservice API)
- `8010` → Octos admin dashboard (optional, for monitoring)

**Persistent volumes:**

| Volume | Purpose |
|--------|---------|
| `./data/pgsql` | PostgreSQL data. Survives `docker compose down`. |
| `./data/octos` | Octos runtime data (sessions, memory). |
| `./data/media` | Media files uploaded through Matrix (images, files). |

**Environment variables (`.env`):**

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DEEPSEEK_API_KEY` | **Yes** | — | Your LLM API key |
| `DB_PASSWORD` | No | `palpo_dev_password` | PostgreSQL password |
| `RUST_LOG` | No | `octos=debug,info` | Log verbosity |

### 3.7 Token Matching Checklist

The most common configuration error is a token mismatch. These values **must be identical** across both files:

| Value | In `octos-registration.yaml` | In `botfather.json` |
|-------|------------------------------|---------------------|
| `as_token` | `as_token: "abc..."` | `"as_token": "abc..."` |
| `hs_token` | `hs_token: "def..."` | `"hs_token": "def..."` |
| `sender_localpart` | `sender_localpart: octosbot` | `"sender_localpart": "octosbot"` |
| `server_name` | `regex: "@octosbot:127\\.0\\.0\\.1:8128"` | `"server_name": "127.0.0.1:8128"` |

If any of these don't match, the bot will not respond to messages. Double-check before filing a bug report!

---

## 4. Using Robrix

This section covers how to use Robrix as a client to connect to your Palpo server and interact with the Octos AI bot.

### 4.1 Getting Robrix

**Download a pre-built release (recommended):**

Download from the [Robrix releases page](https://github.com/Project-Robius-China/robrix2/releases). Available for macOS, Linux, and Windows.

**Or build from source:**

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. On Linux, install dependencies:
   ```bash
   sudo apt-get install libssl-dev libsqlite3-dev pkg-config libxcursor-dev libx11-dev libasound2-dev libpulse-dev libwayland-dev libxkbcommon-dev
   ```
3. Build and run:
   ```bash
   cargo run --release
   ```

For mobile builds (Android/iOS) and packaging for distribution, see the [Robrix README](https://github.com/Project-Robius-China/robrix2#building--running-robrix-on-desktop).

### 4.2 Connecting to Palpo

When you launch Robrix, you'll see the login screen:

<!-- screenshot: login-screen.png — Full login screen -->

The **Homeserver URL** field is at the bottom of the login form. It defaults to `matrix.org` if left empty. To connect to your local Palpo instance:

- **Local deployment:** Enter `http://127.0.0.1:8128`
- **Remote deployment:** Enter `https://your.server.name` (or `http://your-server-ip:8128`)

<!-- screenshot: homeserver-input.png — Homeserver URL input highlighted -->

> **Important:** Robrix requires [Sliding Sync](https://spec.matrix.org/latest/client-server-api/#sliding-sync) support from the homeserver. Palpo supports this natively.

### 4.3 Registration and Login

**First time — Register a new account:**

1. Enter your desired **username** and **password**
2. Enter the password again in the **Confirm password** field (appears for registration)
3. Enter the **Homeserver URL** (e.g., `http://127.0.0.1:8128`)
4. Click **Sign up**

![Register account — enter username, password, and homeserver URL](images/register-account.png)

**Returning — Log in:**

1. Enter your **username** and **password**
2. Enter the **Homeserver URL**
3. Click **Log in**

After successful login, you'll see your room list (empty if this is a fresh account).

<!-- screenshot: room-list.png — Room list after login -->

### 4.4 Interacting with the AI Bot

There are two ways to start chatting with the bot:

#### Method 1: Invite the bot to a room

1. Create a new room or open an existing one
2. Click the **invite** button in the room
3. Enter the bot's Matrix ID: `@octosbot:127.0.0.1:8128` (replace `127.0.0.1:8128` with your `server_name`)
4. The bot will automatically join the room

<!-- screenshot: invite-bot.png — Invite modal with @octosbot:127.0.0.1:8128 entered -->

#### Method 2: Join a room where the bot is present

1. Click the **Join Room** button (or use the room browser)
2. Enter a room alias or ID where the bot has been set up
3. Start chatting

<!-- screenshot: join-room.png — Join room dialog -->

#### Chatting with the bot

Once the bot is in the room, simply type a message and send it. The bot will process your message through the configured LLM and respond.

<!-- screenshot: bot-chat.png — A conversation showing user message and AI bot reply -->

### 4.5 Bot Management (Advanced)

Robrix has built-in support for managing Matrix bots through the BotFather system.

#### Enabling App Service support

1. Open **Settings** in Robrix
2. Navigate to **Bot Settings**
3. Toggle **Enable App Service** on
4. Enter the **BotFather User ID** (e.g., `@octosbot:127.0.0.1:8128`)
5. Click **Save**

<!-- screenshot: bot-settings.png — Bot Settings screen -->

#### Creating child bots

With BotFather enabled, you can create specialized child bots:

1. Use the **Create Bot** dialog
2. Fill in:
   - **Username** — lowercase letters, digits, and underscores only (e.g., `translator_bot`)
   - **Display Name** — human-readable name (e.g., "Translator Bot")
   - **System Prompt** — initial instructions for the bot (e.g., "You are a translator. Translate all messages to English.")
3. Click **Create Bot**

The bot will be created as `@octosbot_<username>:127.0.0.1:8128`.

<!-- screenshot: create-bot.png — Create Bot modal dialog -->

---

## 5. End-to-End Verification

After setting up, run through this checklist to confirm everything works:

### Service Health

```bash
# Check all containers are running
docker compose ps

# Check Palpo logs for startup errors
docker compose logs palpo | tail -20

# Check Octos logs — look for "appservice listening" or similar
docker compose logs octos | tail -20

# Verify Palpo is responding
curl -s http://127.0.0.1:8128/_matrix/client/versions | head -5
```

### Client Connectivity

- [ ] Robrix can connect to `http://127.0.0.1:8128`
- [ ] You can register a new account
- [ ] After login, the room list loads (may be empty)
- [ ] You can create a new room

### Bot Interaction

- [ ] You can invite `@octosbot:127.0.0.1:8128` to a room
- [ ] The bot joins the room (check `docker compose logs octos` if it doesn't)
- [ ] Sending a message triggers a response from the bot
- [ ] The response content makes sense (confirms LLM connection works)

### If something fails

Check the logs in this order — they follow the data flow:

```bash
# 1. Is Palpo receiving messages from Robrix?
docker compose logs palpo --since 1m

# 2. Is Palpo forwarding events to Octos?
docker compose logs palpo --since 1m | grep -i appservice

# 3. Is Octos receiving and processing events?
docker compose logs octos --since 1m

# 4. Is Octos successfully calling the LLM?
docker compose logs octos --since 1m | grep -i -E "deepseek|llm|provider"
```

---

## 6. Troubleshooting

### 6.1 Service Startup Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| `palpo_postgres` won't start | Port 5432 already in use, or corrupt data | Check `docker compose logs palpo_postgres`. Remove `data/pgsql/` to start fresh. |
| `palpo` build fails | Network issue or missing source | Ensure Docker can reach `github.com`. Check `docker compose logs palpo` for build errors. |
| `palpo` crashes on startup | Bad `palpo.toml` syntax or DB connection failure | Check logs. Ensure `palpo_postgres` is healthy first. Verify DB password matches. |
| `octos` build fails | Missing Dockerfile or network issue | Ensure Docker can reach `github.com`. Alternatively, build Octos locally and update `compose.yml` to use a local image. |
| `octos` starts but logs show errors | Invalid `botfather.json` or missing API key | Check JSON syntax. Verify `DEEPSEEK_API_KEY` is set in `.env`. |

### 6.2 Robrix Connection Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| "Cannot connect to server" | Wrong homeserver URL or Palpo not running | Verify Palpo is running (`docker compose ps`). Confirm URL is `http://127.0.0.1:8128`. |
| Login succeeds but no rooms appear | Normal for a fresh account | Create a new room. Rooms will appear as you join or create them. |
| Registration fails | `allow_registration = false` in `palpo.toml`, or server_name mismatch | Check `palpo.toml`. Ensure `allow_registration = true`. |
| "Homeserver does not support Sliding Sync" | Palpo version too old | Rebuild Palpo: `docker compose build --no-cache palpo`. |
| Connection times out | Firewall blocking port 8128 | Check firewall rules. On macOS, allow incoming connections in System Settings. |

### 6.3 Bot Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Bot does not respond to messages | Token mismatch between registration and profile | Verify the [Token Matching Checklist](#37-token-matching-checklist). |
| `Connection refused` in Palpo logs | Octos not running, or wrong `url` in registration YAML | Ensure Octos is running. The `url` must use the Docker service name (`http://octos:8009`), not `localhost`. |
| `User ID not in namespace` | `sender_localpart` doesn't match `namespaces.users` regex | Update the regex in `octos-registration.yaml` to include the bot's full user ID pattern. |
| Bot joins room but gives empty replies | LLM API key invalid or quota exceeded | Check `docker compose logs octos` for API errors. Verify your API key and account balance. |
| Messages from some users are ignored | `allowed_senders` filtering in `botfather.json` | Add the user's Matrix ID to the `allowed_senders` array, or set it to `[]` to allow everyone. |

### 6.4 Useful Debug Commands

```bash
# View real-time logs for all services
docker compose logs -f

# View logs for a specific service
docker compose logs -f palpo
docker compose logs -f octos

# Restart a single service
docker compose restart octos

# Check Palpo's client API
curl http://127.0.0.1:8128/_matrix/client/versions

# Full reset (WARNING: deletes all data)
docker compose down -v
rm -rf data/
docker compose up -d
```

---

## 7. Further Reading

- **Octos Documentation (full):** [octos-org.github.io/octos](https://octos-org.github.io/octos/) — covers all LLM providers, channels (Telegram, Slack, Discord, etc.), skills, memory system, and advanced configuration.
- **Octos Matrix Appservice Guide:** [octos-org/octos#171](https://github.com/octos-org/octos/pull/171) — the original guide this document is based on, with additional context.
- **Palpo:** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) — Palpo homeserver documentation.
- **Robrix:** [Project-Robius-China/robrix2](https://github.com/Project-Robius-China/robrix2) — Robrix client, build instructions, and feature tracker.
- **Matrix Appservice Spec:** [spec.matrix.org — Application Service API](https://spec.matrix.org/latest/application-service-api/) — the Matrix protocol specification for application services.

---

*This guide covers the deployment as of April 2026. For the latest updates, see the respective project repositories.*
