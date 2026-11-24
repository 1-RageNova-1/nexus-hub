use keyring::Entry;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{Manager, State};

const SERVICE_NAME: &str = "com.sasha.nexus";

struct AppDatabase(Mutex<Connection>);

fn get_entry(provider: &str) -> Result<Entry, String> {
    if !["groq", "gemini"].contains(&provider) {
        return Err("Fournisseur non reconnu.".into());
    }
    Entry::new(SERVICE_NAME, provider).map_err(|e| format!("Trousseau système : {e}"))
}

fn db_error(error: impl std::fmt::Display) -> String {
    format!("Base de données Nexus : {error}")
}

#[derive(Deserialize, Serialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ConversationSummary {
    id: String,
    title: String,
    provider: String,
    model: String,
    updated_at: String,
    favorite: bool,
    pinned: bool,
    folder_id: Option<String>,
}

#[derive(Serialize)]
struct Folder {
    id: String,
    name: String,
    created_at: String,
}

#[derive(Serialize)]
struct LibraryItem {
    id: String,
    name: String,
    mime_type: String,
    created_at: String,
    content: String,
}

#[tauri::command]
fn save_api_key(provider: String, api_key: String) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err("La clé API ne peut pas être vide.".into());
    }
    get_entry(&provider)?.set_password(api_key.trim())
        .map_err(|e| format!("Impossible d'enregistrer la clé : {e}"))
}

#[tauri::command]
fn has_api_key(provider: String) -> Result<bool, String> {
    match get_entry(&provider)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(format!("Erreur du trousseau : {e}")),
    }
}

#[tauri::command]
fn delete_api_key(provider: String) -> Result<(), String> {
    match get_entry(&provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Impossible de supprimer la clé : {e}")),
    }
}

#[tauri::command]
fn save_conversation(
    db: State<'_, AppDatabase>,
    id: String,
    title: String,
    provider: String,
    model: String,
    messages: Vec<ChatMessage>,
    favorite: bool,
    pinned: bool,
    folder_id: Option<String>,
) -> Result<(), String> {
    let connection = db.0.lock().map_err(db_error)?;
    let serialized = serde_json::to_string(&messages).map_err(db_error)?;
    connection.execute(
        "INSERT INTO conversations (id,title,provider,model,messages,favorite,pinned,folder_id,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,datetime('now'))
         ON CONFLICT(id) DO UPDATE SET title=excluded.title,provider=excluded.provider,model=excluded.model,
         messages=excluded.messages,favorite=excluded.favorite,pinned=excluded.pinned,folder_id=excluded.folder_id,
         updated_at=datetime('now')",
        params![id,title,provider,model,serialized,favorite,pinned,folder_id],
    ).map_err(db_error)?;
    Ok(())
}

#[tauri::command]
fn list_conversations(db: State<'_, AppDatabase>, query: Option<String>) -> Result<Vec<ConversationSummary>, String> {
    let connection = db.0.lock().map_err(db_error)?;
    let mut statement = connection.prepare(
        "SELECT id,title,provider,model,updated_at,favorite,pinned,folder_id FROM conversations
         WHERE (?1 IS NULL OR title LIKE '%' || ?1 || '%') ORDER BY pinned DESC, updated_at DESC"
    ).map_err(db_error)?;
    let rows = statement.query_map(params![query], |row| Ok(ConversationSummary {
        id: row.get(0)?, title: row.get(1)?, provider: row.get(2)?, model: row.get(3)?,
        updated_at: row.get(4)?, favorite: row.get(5)?, pinned: row.get(6)?, folder_id: row.get(7)?,
    })).map_err(db_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
}

#[tauri::command]
fn get_conversation(db: State<'_, AppDatabase>, id: String) -> Result<Value, String> {
    let connection = db.0.lock().map_err(db_error)?;
    let row: Option<(String,String,String,String,bool,bool,Option<String>)> = connection.query_row(
        "SELECT title,provider,model,messages,favorite,pinned,folder_id FROM conversations WHERE id=?1",
        params![id],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))
    ).optional().map_err(db_error)?;
    let (title,provider,model,messages,favorite,pinned,folder_id) = row.ok_or("Conversation introuvable.")?;
    Ok(json!({"id":id,"title":title,"provider":provider,"model":model,"messages":serde_json::from_str::<Value>(&messages).map_err(db_error)?,"favorite":favorite,"pinned":pinned,"folder_id":folder_id}))
}

#[tauri::command]
fn delete_conversation(db: State<'_, AppDatabase>, id: String) -> Result<(), String> {
    db.0.lock().map_err(db_error)?.execute("DELETE FROM conversations WHERE id=?1", params![id]).map_err(db_error)?;
    Ok(())
}

#[tauri::command]
fn list_folders(db: State<'_, AppDatabase>) -> Result<Vec<Folder>, String> {
    let connection = db.0.lock().map_err(db_error)?;
    let mut statement = connection.prepare("SELECT id,name,created_at FROM folders ORDER BY name COLLATE NOCASE").map_err(db_error)?;
    let rows = statement.query_map([], |r| Ok(Folder {id:r.get(0)?,name:r.get(1)?,created_at:r.get(2)?})).map_err(db_error)?;
    rows.collect::<Result<Vec<_>,_>>().map_err(db_error)
}

#[tauri::command]
fn create_folder(db: State<'_, AppDatabase>, id: String, name: String) -> Result<(), String> {
    if name.trim().is_empty() { return Err("Le nom du dossier est vide.".into()); }
    db.0.lock().map_err(db_error)?.execute("INSERT INTO folders(id,name) VALUES(?1,?2)", params![id,name.trim()]).map_err(db_error)?;
    Ok(())
}

#[tauri::command]
fn delete_folder(db: State<'_, AppDatabase>, id: String) -> Result<(), String> {
    let connection = db.0.lock().map_err(db_error)?;
    connection.execute("UPDATE conversations SET folder_id=NULL WHERE folder_id=?1", params![id]).map_err(db_error)?;
    connection.execute("DELETE FROM folders WHERE id=?1", params![id]).map_err(db_error)?;
    Ok(())
}

#[tauri::command]
fn save_library_item(db: State<'_, AppDatabase>, id: String, name: String, mime_type: String, content: String) -> Result<(), String> {
    db.0.lock().map_err(db_error)?.execute(
        "INSERT INTO library_items(id,name,mime_type,content) VALUES(?1,?2,?3,?4)",
        params![id,name,mime_type,content]
    ).map_err(db_error)?;
    Ok(())
}

#[tauri::command]
fn list_library_items(db: State<'_, AppDatabase>) -> Result<Vec<LibraryItem>, String> {
    let connection = db.0.lock().map_err(db_error)?;
    let mut statement = connection.prepare("SELECT id,name,mime_type,created_at,content FROM library_items ORDER BY created_at DESC").map_err(db_error)?;
    let rows = statement.query_map([], |r| Ok(LibraryItem {id:r.get(0)?,name:r.get(1)?,mime_type:r.get(2)?,created_at:r.get(3)?,content:r.get(4)?})).map_err(db_error)?;
    rows.collect::<Result<Vec<_>,_>>().map_err(db_error)
}

#[tauri::command]
fn delete_library_item(db: State<'_, AppDatabase>, id: String) -> Result<(), String> {
    db.0.lock().map_err(db_error)?.execute("DELETE FROM library_items WHERE id=?1", params![id]).map_err(db_error)?;
    Ok(())
}

#[tauri::command]
fn set_preference(db: State<'_, AppDatabase>, key: String, value: String) -> Result<(), String> {
    db.0.lock().map_err(db_error)?.execute(
        "INSERT INTO preferences(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key,value]
    ).map_err(db_error)?;
    Ok(())
}

#[tauri::command]
fn get_preferences(db: State<'_, AppDatabase>) -> Result<Value, String> {
    let connection = db.0.lock().map_err(db_error)?;
    let mut statement = connection.prepare("SELECT key,value FROM preferences").map_err(db_error)?;
    let rows = statement.query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(db_error)?;
    let mut result = serde_json::Map::new();
    for row in rows { let (key,value) = row.map_err(db_error)?; result.insert(key,Value::String(value)); }
    Ok(Value::Object(result))
}

#[tauri::command]
async fn send_chat_message(provider: String, model: String, messages: Vec<ChatMessage>) -> Result<String, String> {
    if messages.is_empty() { return Err("La conversation est vide.".into()); }
    let api_key = get_entry(&provider)?.get_password().map_err(|_| format!("Aucune clé API enregistrée pour {provider}."))?;
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(120)).build().map_err(|e| e.to_string())?;
    let (url, body, gemini) = match provider.as_str() {
        "groq" => ("https://api.groq.com/openai/v1/chat/completions".to_string(),
            json!({"model":model,"messages":messages,"temperature":0.7}), false),
        "gemini" => {
            let mut contents = Vec::new();
            let mut system = Vec::new();
            for message in &messages {
                match message.role.as_str() {
                    "system" => system.push(message.content.clone()),
                    "user" => contents.push(json!({"role":"user","parts":[{"text":message.content}]})),
                    "assistant" => contents.push(json!({"role":"model","parts":[{"text":message.content}]})),
                    _ => return Err("Rôle de message non reconnu.".into()),
                }
            }
            let mut body = json!({"contents":contents,"generationConfig":{"temperature":0.7}});
            if !system.is_empty() { body["systemInstruction"] = json!({"parts":[{"text":system.join("\n")} ]}); }
            (format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"),body,true)
        }
        _ => return Err("Fournisseur non reconnu.".into()),
    };
    let mut request = client.post(url).header("Content-Type","application/json");
    request = if gemini { request.header("x-goog-api-key",&api_key) } else { request.bearer_auth(&api_key) };
    let response = request.json(&body).send().await.map_err(|e| format!("Erreur réseau : {e}"))?;
    let status = response.status();
    let payload: Value = response.json().await.map_err(|e| format!("Réponse API invalide : {e}"))?;
    if !status.is_success() {
        let detail = payload["error"]["message"].as_str().or_else(|| payload["message"].as_str()).unwrap_or("Erreur inconnue du fournisseur.");
        return Err(format!("API {provider} ({status}): {detail}"));
    }
    let answer = if gemini {
        payload["candidates"][0]["content"]["parts"].as_array().map(|parts| parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default()
    } else {
        payload["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string()
    };
    if answer.is_empty() { return Err("Le fournisseur n'a retourné aucune réponse textuelle.".into()); }
    Ok(answer)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let mut data_dir: PathBuf = app.path().app_data_dir()?;
            fs::create_dir_all(&data_dir)?;
            data_dir.push("nexus.sqlite3");
            let connection = Connection::open(data_dir)?;
            connection.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA foreign_keys=ON;
                 CREATE TABLE IF NOT EXISTS conversations(
                    id TEXT PRIMARY KEY,title TEXT NOT NULL,provider TEXT NOT NULL,model TEXT NOT NULL,
                    messages TEXT NOT NULL,favorite INTEGER NOT NULL DEFAULT 0,pinned INTEGER NOT NULL DEFAULT 0,
                    folder_id TEXT,updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
                 CREATE TABLE IF NOT EXISTS folders(id TEXT PRIMARY KEY,name TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
                 CREATE TABLE IF NOT EXISTS library_items(id TEXT PRIMARY KEY,name TEXT NOT NULL,mime_type TEXT NOT NULL,content TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
                 CREATE TABLE IF NOT EXISTS preferences(key TEXT PRIMARY KEY,value TEXT NOT NULL);"
            )?;
            app.manage(AppDatabase(Mutex::new(connection)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_api_key,has_api_key,delete_api_key,send_chat_message,
            save_conversation,list_conversations,get_conversation,delete_conversation,
            list_folders,create_folder,delete_folder,save_library_item,list_library_items,delete_library_item,
            set_preference,get_preferences
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
