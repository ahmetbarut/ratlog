//! CLI login to Ratlog Web: open browser, verify token, save locally.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

const DEFAULT_APP_URL: &str = "https://ratlog.info";
const RATLOG_WEB_URL_ENV: &str = "RATLOG_WEB_URL";

fn app_url() -> String {
    std::env::var(RATLOG_WEB_URL_ENV).unwrap_or_else(|_| DEFAULT_APP_URL.to_string())
}

fn token_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("ratlog").join("token"))
}

/// Open default browser to the given URL.
fn open_browser(url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(target_os = "macos")]
    Command::new("open").arg(url).spawn()?;

    #[cfg(target_os = "windows")]
    Command::new("cmd").args(["/C", "start", url]).spawn()?;

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Command::new("xdg-open").arg(url).spawn()?;

    Ok(())
}

/// Read token from stdin.
fn read_token_from_stdin() -> Result<String, io::Error> {
    print!("Token'ı yapıştırın: ");
    io::stdout().flush()?;
    let mut token = String::new();
    io::stdin().read_line(&mut token)?;
    Ok(token.trim().to_string())
}

/// Verify token via GET /api/user.
async fn verify_token(
    base_url: &str,
    token: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/api/user", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    if response.status().is_success() {
        let user: serde_json::Value = response.json().await?;
        Ok(user)
    } else {
        Err(format!("Token doğrulama başarısız: {}", response.status()).into())
    }
}

/// Save token to config file (~/.config/ratlog/token).
fn save_token(token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = token_path().ok_or("Config dizini bulunamadı")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, token)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Run the login flow: open browser, prompt for token, verify, save.
pub async fn run() -> color_eyre::Result<()> {
    let base_url = app_url();
    let login_url = format!("{}/cli-login", base_url.trim_end_matches('/'));

    println!("Ratlog Web CLI Girişi");
    println!();
    println!("Tarayıcı açılıyor: {}", login_url);

    if let Err(e) = open_browser(&login_url) {
        eprintln!("Tarayıcı açılamadı: {}", e);
        eprintln!("Lütfen şu adresi manuel açın: {}", login_url);
    }

    println!();
    println!("Tarayıcıda giriş yapın ve token'ı kopyalayın.");
    println!();

    let token = read_token_from_stdin()?;
    if token.is_empty() {
        color_eyre::eyre::bail!("Token boş bırakılamaz.");
    }

    println!("Token doğrulanıyor...");
    let user = verify_token(&base_url, &token)
        .await
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    save_token(&token).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    let email = user["email"].as_str().unwrap_or("?");
    println!();
    println!("✓ Giriş başarılı: {}", email);
    println!("Token kaydedildi: {:?}", token_path().unwrap_or_default());

    Ok(())
}

/// Load saved token from config file. Returns None if not found or invalid.
pub fn load_token() -> Option<String> {
    let path = token_path()?;
    let s = fs::read_to_string(&path).ok()?;
    let token = s.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Response from POST /api/logs.
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct ShareLogResponse {
    pub id: String,
    pub url: String,
    #[serde(alias = "view_url")]
    pub view_url: Option<String>,
    #[serde(default)]
    pub delete_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub line_count: Option<u64>,
}

/// Share log content to Ratlog Web. Returns the share URL or error.
pub async fn share_log(
    content: &str,
    is_public: bool,
) -> Result<ShareLogResponse, Box<dyn std::error::Error + Send + Sync>> {
    let token = load_token().ok_or("Giriş yapılmamış. Önce 'ratlog login' çalıştırın.")?;
    let base_url = app_url();
    let url = format!("{}/api/logs", base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "content": content,
            "is_public": is_public
        }))
        .send()
        .await?;

    if response.status().as_u16() == 201 {
        let body: ShareLogResponse = response.json().await?;
        Ok(body)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Log paylaşımı başarısız ({}): {}", status, body).into())
    }
}
