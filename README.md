# Nexus

Nexus is a native desktop AI workspace built with Tauri 2, Rust and vanilla HTML/CSS/JavaScript.

## Features

- Chat with Groq and OpenRouter using your own API keys.
- API keys stored in the operating system credential manager.
- SQLite-backed conversation history, search, favorites and pinned conversations.
- Organize conversations into folders.
- Import text-based resources into a local library and reuse them in chats.
- Context presets: Quick, Smart, Full, Project and Custom.
- JSON backup export and restore.
- Automatic provider fallback when a provider reports rate limiting or quota exhaustion.

## Data and privacy

Conversation data, folders, library content and preferences are stored locally in the application data directory using SQLite. Provider credentials are stored in the operating system keyring. Requests are sent directly from the desktop application to the selected provider API.
