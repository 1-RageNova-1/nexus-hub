const providers = {
    groq: {
        name: "Groq",
        models: [
            {
                id: "openai/gpt-oss-20b",
                name: "GPT-OSS 20B"
            },
            {
                id: "openai/gpt-oss-120b",
                name: "GPT-OSS 120B"
            },
            {
                id: "qwen/qwen3.8-27b",
                name: "Qwen 3.8 27B"
            }
        ],
        defaultModel: "openai/gpt-oss-20b"
    },
    gemini: {
        name: "Gemini",
        models: [
            {
                id: "gemini-3.8-flash",
                name: "Gemini 3.8 Flash"
            },
            {
                id: "gemini-3.1-flash-lite",
                name: "Gemini 3.1 Flash-Lite"
            }
        ],
        defaultModel: "gemini-3.8-flash"
    }
};

function hideLoadingScreen() {
    const loadingScreen = document.getElementById("loading-screen");

    if (!loadingScreen) return;

    loadingScreen.classList.add("loading-screen-hidden");

    setTimeout(() => {
        loadingScreen.remove();
    }, 600);
}

const invoke = window.__TAURI__?.core?.invoke;

const currentProvider = document.getElementById("current-provider");
const modelSelect = document.getElementById("model-select");
const messageInput = document.getElementById("message-input");
const chatArea = document.getElementById("chat-area");
const welcome = document.getElementById("welcome");
const messagesContainer = document.getElementById("messages");
const settingsOverlay = document.getElementById("settings-overlay");
const apiProvider = document.getElementById("api-provider");
const apiKeyInput = document.getElementById("api-key-input");
const keyStatus = document.getElementById("key-status");
const welcomeHint = document.getElementById("welcome-hint");

let activeProvider = "groq";
let conversation = [];
let isSending = false;

const apiKeysStatus = {
    groq: false,
    gemini: false
};

const modelPreferences = {
    groq: providers.groq.defaultModel,
    gemini: providers.gemini.defaultModel
};

function addStyles() {
    const style = document.createElement("style");

    style.textContent = `
        .messages {
            display: none;
            flex-direction: column;
            gap: 18px;
            width: 100%;
            max-width: 900px;
            margin: 0 auto;
            padding: 30px 24px;
            overflow-y: auto;
            flex: 1;
        }

        .message {
            display: flex;
            flex-direction: column;
            gap: 7px;
            max-width: 85%;
            animation: messageAppear .2s ease;
        }

        .message.user {
            align-self: flex-end;
        }

        .message.assistant {
            align-self: flex-start;
        }

        .message-role {
            font-size: 12px;
            opacity: .55;
        }

        .message-content {
            padding: 13px 16px;
            border-radius: 14px;
            background: rgba(255,255,255,.055);
            color: #eeeef4;
            line-height: 1.65;
            white-space: pre-wrap;
            overflow-wrap: anywhere;
            font-size: 14px;
        }

        .message.user .message-content {
            background: rgba(120,100,255,.18);
        }

        .message.error .message-content {
            background: rgba(255,70,70,.12);
            color: #ffaaaa;
        }

        .message-loading {
            opacity: .6;
            animation: loadingPulse 1.2s infinite;
        }

        .model-select {
            max-width: 220px;
            padding: 8px 12px;
            border-radius: 9px;
            color: #eee;
            background: #17171f;
            border: 1px solid rgba(255,255,255,.1);
            outline: none;
        }

        .provider-status.connected {
            background: #42d88a;
            box-shadow: 0 0 8px rgba(66,216,138,.35);
        }

        .provider-status.disconnected {
            background: #777;
        }

        .provider-status.limited {
            background: #ffad42;
        }

        .settings-overlay {
            display: none;
            position: fixed;
            inset: 0;
            z-index: 1000;
            align-items: center;
            justify-content: center;
            padding: 20px;
            background: rgba(0,0,0,.68);
            backdrop-filter: blur(12px);
        }

        .settings-overlay.visible {
            display: flex;
        }

        .settings-modal {
            width: 100%;
            max-width: 460px;
            border: 1px solid rgba(255,255,255,.1);
            border-radius: 18px;
            background: #15151d;
            color: #eeeef4;
            box-shadow: 0 25px 80px rgba(0,0,0,.5);
            overflow: hidden;
            animation: messageAppear .2s ease;
        }

        .settings-header {
            display: flex;
            align-items: flex-start;
            justify-content: space-between;
            padding: 23px;
            border-bottom: 1px solid rgba(255,255,255,.08);
        }

        .settings-header h2 {
            margin: 0 0 6px;
            font-size: 19px;
        }

        .settings-header p {
            margin: 0;
            font-size: 13px;
            opacity: .55;
        }

        .settings-close {
            border: 0;
            background: transparent;
            color: #aaa;
            font-size: 25px;
            cursor: pointer;
        }

        .settings-content {
            display: flex;
            flex-direction: column;
            gap: 11px;
            padding: 23px;
        }

        .settings-content label {
            margin-top: 7px;
            font-size: 13px;
            opacity: .75;
        }

        .settings-content select,
        .settings-content input {
            width: 100%;
            box-sizing: border-box;
            padding: 12px;
            border: 1px solid rgba(255,255,255,.1);
            border-radius: 9px;
            outline: none;
            background: #20202a;
            color: #eee;
            font: inherit;
        }

        .settings-content input:focus,
        .settings-content select:focus {
            border-color: #8172ff;
        }

        .key-status {
            min-height: 18px;
            font-size: 12px;
            opacity: .65;
        }

        .settings-actions {
            display: flex;
            justify-content: space-between;
            gap: 10px;
            margin-top: 8px;
        }

        .settings-actions button {
            padding: 11px 14px;
            border: 0;
            border-radius: 9px;
            cursor: pointer;
            font: inherit;
            font-size: 13px;
        }

        .primary-button {
            background: #7968ed;
            color: white;
        }

        .danger-button {
            background: rgba(255,70,70,.12);
            color: #ff9999;
        }

        .security-note {
            margin: 9px 0 0;
            font-size: 11px;
            line-height: 1.6;
            opacity: .45;
        }

        .welcome-hint {
            margin-top: 24px;
            font-size: 12px;
            opacity: .55;
        }

        .send-button:disabled {
            opacity: .4;
            cursor: not-allowed;
        }

        @keyframes messageAppear {
            from {
                opacity: 0;
                transform: translateY(6px);
            }
            to {
                opacity: 1;
                transform: translateY(0);
            }
        }

        @keyframes loadingPulse {
            0%, 100% {
                opacity: .4;
            }
            50% {
                opacity: .9;
            }
        }
    `;

    document.head.appendChild(style);
}

function setStatus(provider, status) {
    const indicator = document.getElementById(`status-${provider}`);

    if (!indicator) return;

    indicator.classList.remove(
        "connected",
        "disconnected",
        "limited"
    );

    indicator.classList.add(status);
}

function updateWelcomeHint() {
    const hasAnyKey = Object.values(apiKeysStatus).some(Boolean);

    welcomeHint.textContent = hasAnyKey
        ? "Sélectionne un assistant et commence une conversation."
        : "Configure une clé API dans les paramètres pour commencer.";
}

async function refreshApiStatuses() {
    for (const provider of Object.keys(providers)) {
        try {
            const exists = await invoke("has_api_key", {
                provider
            });

            apiKeysStatus[provider] = exists;

            setStatus(
                provider,
                exists ? "connected" : "disconnected"
            );
        } catch (error) {
            console.error(`Erreur de vérification ${provider}:`, error);

            apiKeysStatus[provider] = false;
            setStatus(provider, "disconnected");
        }
    }

    updateWelcomeHint();
}

function updateModelSelect() {
    const config = providers[activeProvider];

    modelSelect.innerHTML = "";

    config.models.forEach(model => {
        const option = document.createElement("option");

        option.value = model.id;
        option.textContent = model.name;

        modelSelect.appendChild(option);
    });

    modelSelect.value = modelPreferences[activeProvider];
}

function updateProviderButtons() {
    document.querySelectorAll("[data-provider]").forEach(button => {
        button.classList.toggle(
            "active",
            button.dataset.provider === activeProvider
        );
    });
}

function switchProvider(provider, clearConversation = true) {
    if (!providers[provider]) return;

    activeProvider = provider;

    currentProvider.textContent = providers[provider].name;

    updateProviderButtons();
    updateModelSelect();

    if (clearConversation) {
        conversation = [];
        messagesContainer.innerHTML = "";
        messagesContainer.style.display = "none";
        welcome.style.display = "flex";
    }

    messageInput.placeholder = `Écrire à ${providers[provider].name}...`;
}

function renderMessage(role, content, error = false) {
    welcome.style.display = "none";
    messagesContainer.style.display = "flex";

    const message = document.createElement("div");

    message.className = `message ${role}${error ? " error" : ""}`;

    const label = document.createElement("span");

    label.className = "message-role";
    label.textContent = role === "user"
        ? "Toi"
        : providers[activeProvider].name;

    const text = document.createElement("div");

    text.className = "message-content";
    text.textContent = content;

    message.append(label, text);
    messagesContainer.appendChild(message);

    messagesContainer.scrollTop = messagesContainer.scrollHeight;

    return message;
}

function showSettings() {
    settingsOverlay.classList.add("visible");
    apiProvider.value = activeProvider;
    apiKeyInput.value = "";

    updateKeyStatus();
}

function hideSettings() {
    settingsOverlay.classList.remove("visible");
    apiKeyInput.value = "";
}

async function updateKeyStatus() {
    const provider = apiProvider.value;

    keyStatus.textContent = "Vérification...";

    try {
        const exists = await invoke("has_api_key", {
            provider
        });

        apiKeysStatus[provider] = exists;

        keyStatus.textContent = exists
            ? "● Une clé API est enregistrée."
            : "○ Aucune clé API enregistrée.";

        setStatus(
            provider,
            exists ? "connected" : "disconnected"
        );

        updateWelcomeHint();
    } catch (error) {
        keyStatus.textContent = `Erreur : ${error}`;
    }
}

async function saveApiKey() {
    const provider = apiProvider.value;
    const apiKey = apiKeyInput.value.trim();

    if (!apiKey) {
        keyStatus.textContent = "Entre une clé API avant de continuer.";
        return;
    }

    const button = document.getElementById("save-api-key");

    button.disabled = true;
    button.textContent = "Enregistrement...";

    try {
        await invoke("save_api_key", {
            provider,
            apiKey
        });

        apiKeysStatus[provider] = true;
        apiKeyInput.value = "";

        keyStatus.textContent = "Clé API enregistrée avec succès.";

        setStatus(provider, "connected");
        updateWelcomeHint();
    } catch (error) {
        keyStatus.textContent = `Erreur : ${error}`;
    } finally {
        button.disabled = false;
        button.textContent = "Enregistrer la clé";
    }
}

async function deleteApiKey() {
    const provider = apiProvider.value;

    try {
        await invoke("delete_api_key", {
            provider
        });

        apiKeysStatus[provider] = false;
        apiKeyInput.value = "";

        keyStatus.textContent = "Clé API supprimée.";

        setStatus(provider, "disconnected");
        updateWelcomeHint();
    } catch (error) {
        keyStatus.textContent = `Erreur : ${error}`;
    }
}

function isQuotaError(error) {
    const text = String(error).toLowerCase();

    return (
        text.includes("429") ||
        text.includes("quota") ||
        text.includes("rate limit") ||
        text.includes("resource_exhausted") ||
        text.includes("too many requests")
    );
}

async function requestProvider(provider, messages) {
    return invoke("send_chat_message", {
        provider,
        model: modelPreferences[provider],
        messages
    });
}

async function sendMessage() {
    if (isSending) return;

    const message = messageInput.value.trim();

    if (!message) return;

    if (!apiKeysStatus[activeProvider]) {
        showSettings();

        keyStatus.textContent =
            "Ajoute d'abord une clé API pour ce fournisseur.";

        return;
    }

    isSending = true;

    const sendButton = document.getElementById("send-message");

    sendButton.disabled = true;

    conversation.push({
        role: "user",
        content: message
    });

    renderMessage("user", message);

    messageInput.value = "";

    let loadingMessage = renderMessage(
        "assistant",
        "Réflexion en cours..."
    );

    loadingMessage.classList.add("message-loading");

    try {
        let answer;

        try {
            answer = await requestProvider(
                activeProvider,
                conversation
            );
        } catch (error) {
            if (!isQuotaError(error)) {
                throw error;
            }

            const previousProvider = activeProvider;

            setStatus(previousProvider, "limited");

            const alternative = Object.keys(providers).find(
                provider =>
                    provider !== previousProvider &&
                    apiKeysStatus[provider]
            );

            if (!alternative) {
                throw new Error(
                    "Quota dépassé. Aucun autre fournisseur configuré."
                );
            }

            loadingMessage.remove();

            switchProvider(alternative, false);

            renderMessage(
                "assistant",
                `Limite atteinte chez ${providers[previousProvider].name}. Passage automatique vers ${providers[alternative].name}.`
            );

            loadingMessage = renderMessage(
                "assistant",
                "Réflexion en cours..."
            );

            loadingMessage.classList.add("message-loading");

            answer = await requestProvider(
                alternative,
                conversation
            );
        }

        loadingMessage.remove();

        conversation.push({
            role: "assistant",
            content: answer
        });

        renderMessage("assistant", answer);

    } catch (error) {
        loadingMessage.remove();

        renderMessage(
            "assistant",
            String(error),
            true
        );

        console.error("Erreur d'envoi :", error);

    } finally {
        isSending = false;
        sendButton.disabled = false;
        messageInput.focus();
    }
}

function newChat() {
    conversation = [];

    messagesContainer.innerHTML = "";
    messagesContainer.style.display = "none";

    welcome.style.display = "flex";

    messageInput.value = "";
    messageInput.focus();
}

function initializeEvents() {
    document.querySelectorAll(".provider, .welcome-card").forEach(button => {
        button.addEventListener("click", () => {
            switchProvider(button.dataset.provider);
        });
    });

    document.getElementById("new-chat").addEventListener("click", newChat);

    document.getElementById("top-new-chat").addEventListener("click", newChat);

    document.getElementById("send-message").addEventListener("click", sendMessage);

    messageInput.addEventListener("keydown", event => {
        if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });

    modelSelect.addEventListener("change", () => {
        modelPreferences[activeProvider] = modelSelect.value;
    });

    document.getElementById("open-settings").addEventListener("click", showSettings);

    document.getElementById("top-settings").addEventListener("click", showSettings);

    document.getElementById("composer-settings").addEventListener("click", showSettings);

    document.getElementById("close-settings").addEventListener("click", hideSettings);

    settingsOverlay.addEventListener("click", event => {
        if (event.target === settingsOverlay) {
            hideSettings();
        }
    });

    apiProvider.addEventListener("change", updateKeyStatus);

    document.getElementById("save-api-key").addEventListener("click", saveApiKey);

    document.getElementById("delete-api-key").addEventListener("click", deleteApiKey);

    document.getElementById("nav-conversations").addEventListener("click", newChat);

    document.getElementById("nav-history").addEventListener("click", () => {
        alert("L'historique persistant sera ajouté dans une prochaine étape.");
    });

    document.getElementById("nav-library").addEventListener("click", () => {
        alert("La bibliothèque sera ajoutée dans une prochaine étape.");
    });
}

async function initialize() {
    addStyles();

    try {
        if (!invoke) {
            throw new Error("API Tauri introuvable.");
        }

        initializeEvents();
        updateModelSelect();
        updateProviderButtons();

        await refreshApiStatuses();
    } catch (error) {
        console.error("Erreur d'initialisation :", error);
        welcomeHint.textContent = `Erreur d'initialisation : ${error}`;
    } finally {
        hideLoadingScreen();
    }
}

initialize();
