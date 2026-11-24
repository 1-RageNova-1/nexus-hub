# Nexus

Nexus is a native desktop AI workspace built with Tauri 2, Rust and vanilla HTML/CSS/JavaScript.

## Features

- Chat with Groq and Gemini using your own API keys.
- API keys stored in the operating system credential manager.
- SQLite-backed conversation history, search, favorites and pinned conversations.
- Organize conversations into folders.
- Import text-based resources into a local library and reuse them in chats.
- Context presets: Quick, Smart, Full, Project and Custom.
- JSON backup export and restore.
- Automatic provider fallback when a provider reports rate limiting or quota exhaustion.

## Development

Requirements: Node.js 22+, Rust stable, Tauri 2 system dependencies.

```sh
npm install
cargo tauri dev
```

## Linux package

```sh
cargo tauri build --bundles deb
```

GitHub Actions builds the Debian package for Linux x86_64 and publishes it as a workflow artifact.

## Data and privacy

Conversation data, folders, library content and preferences are stored locally in the application data directory using SQLite. Provider credentials are stored in the operating system keyring. Requests are sent directly from the desktop application to the selected provider API.
