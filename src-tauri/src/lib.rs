use keyring::Entry;
use serde::Deserialize;
use serde_json::{Value, json};

const SERVICE_NAME: &str = "com.sasha.nexus";

fn get_entry(provider: &str) -> Result<Entry, String> {
    if !["groq", "gemini", "mistral"].contains(&provider) {
        return Err("Fournisseur non reconnu.".into());
    }

    Entry::new(SERVICE_NAME, provider).map_err(|error| format!("Erreur du trousseau : {error}"))
}

#[derive(Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn save_api_key(provider: String, api_key: String) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err("La clé API ne peut pas être vide.".into());
    }

    get_entry(&provider)?
        .set_password(api_key.trim())
        .map_err(|error| format!("Impossible d'enregistrer la clé : {error}"))
}

#[tauri::command]
fn has_api_key(provider: String) -> Result<bool, String> {
    match get_entry(&provider)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("Erreur du trousseau : {error}")),
    }
}

#[tauri::command]
fn delete_api_key(provider: String) -> Result<(), String> {
    match get_entry(&provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Impossible de supprimer la clé : {error}")),
    }
}

#[tauri::command]
async fn send_chat_message(
    provider: String,
    model: String,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    if messages.is_empty() {
        return Err("La conversation est vide.".into());
    }

    let api_key = get_entry(&provider)?
        .get_password()
        .map_err(|_| format!("Aucune clé API enregistrée pour {}.", provider))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|error| error.to_string())?;

    let (url, request, gemini): (String, Value, bool) = match provider.as_str() {
        "groq" | "mistral" => {
            let url = if provider == "groq" {
                "https://api.groq.com/openai/v1/chat/completions"
            } else {
                "https://api.mistral.ai/v1/chat/completions"
            };

            let request_messages: Vec<Value> = messages
                .iter()
                .map(|message| {
                    json!({
                        "role": message.role,
                        "content": message.content
                    })
                })
                .collect();

            (
                url.to_string(),
                json!({
                    "model": model,
                    "messages": request_messages
                }),
                false,
            )
        }

        "gemini" => {
            let mut contents = Vec::new();
            let mut system_instruction = Vec::new();

            for message in &messages {
                match message.role.as_str() {
                    "system" => system_instruction.push(message.content.clone()),
                    "user" => contents.push(json!({
                        "role": "user",
                        "parts": [{"text": message.content}]
                    })),
                    "assistant" => contents.push(json!({
                        "role": "model",
                        "parts": [{"text": message.content}]
                    })),
                    _ => return Err("Rôle de message non reconnu.".into()),
                }
            }

            let mut body = json!({
                "contents": contents
            });

            if !system_instruction.is_empty() {
                body["systemInstruction"] = json!({
                    "parts": [{
                        "text": system_instruction.join("\n")
                    }]
                });
            }

            (
                format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                    model
                ),
                body,
                true,
            )
        }

        _ => return Err("Fournisseur non reconnu.".into()),
    };

    let mut request_builder = client.post(url).header("Content-Type", "application/json");

    request_builder = if gemini {
        request_builder.header("x-goog-api-key", &api_key)
    } else {
        request_builder.bearer_auth(&api_key)
    };

    let response = request_builder
        .json(&request)
        .send()
        .await
        .map_err(|error| format!("Erreur réseau : {error}"))?;

    let status = response.status();

    let response_body: Value = response
        .json()
        .await
        .map_err(|error| format!("Réponse API invalide : {error}"))?;

    if !status.is_success() {
        let detail = response_body["error"]["message"]
            .as_str()
            .unwrap_or("Erreur inconnue du fournisseur.");

        return Err(format!("Erreur API {} : {} ({})", provider, detail, status));
    }

    let answer = if gemini {
        response_body["candidates"][0]["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| part["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default()
    } else {
        response_body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };

    if answer.is_empty() {
        return Err("Le fournisseur n'a retourné aucune réponse textuelle.".into());
    }

    Ok(answer)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            save_api_key,
            has_api_key,
            delete_api_key,
            send_chat_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
