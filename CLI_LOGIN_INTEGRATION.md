# Ratlog Web CLI Login Entegrasyonu

Bu dokümantasyon, Ratlog Web'in CLI login akışını başka bir CLI uygulamasına (ör. Rust ratlog TUI) nasıl entegre edeceğinizi açıklar.

## Akış Özeti

1. CLI uygulaması kullanıcıya `ratlog login` komutunu çalıştırır
2. CLI, tarayıcıyı `https://ratlog-web.example.com/cli-login` adresine açar
3. Kullanıcı tarayıcıda giriş yapar (gerekirse)
4. Tarayıcıda bir token gösterilir
5. Kullanıcı token'ı kopyalayıp CLI'ye yapıştırır
6. CLI token'ı doğrular ve yerel olarak saklar
7. Sonraki API isteklerinde bu token kullanılır

---

## 1. Tarayıcıyı Açma

CLI uygulamanız, kullanıcının tarayıcısını Ratlog Web'in CLI login sayfasına yönlendirmelidir.

### URL Formatı

```
{APP_URL}/cli-login
```

Örnek:
- `https://ratlog.example.com/cli-login`
- `http://localhost:8000/cli-login` (geliştirme)

### Platform Bazlı Tarayıcı Açma

#### Rust

```rust
use std::process::Command;

fn open_browser(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    Command::new("open").arg(url).spawn()?;
    
    #[cfg(target_os = "windows")]
    Command::new("cmd")
        .args(["/C", "start", url])
        .spawn()?;
    
    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(url).spawn()?;
    
    Ok(())
}

// Kullanım
open_browser("https://ratlog.example.com/cli-login")?;
```

#### Python

```python
import webbrowser

def open_browser(url: str):
    webbrowser.open(url)

# Kullanım
open_browser("https://ratlog.example.com/cli-login")
```

#### Go

```go
package main

import (
    "os/exec"
    "runtime"
)

func openBrowser(url string) error {
    var cmd *exec.Cmd
    switch runtime.GOOS {
    case "darwin":
        cmd = exec.Command("open", url)
    case "windows":
        cmd = exec.Command("cmd", "/C", "start", url)
    default: // Linux
        cmd = exec.Command("xdg-open", url)
    }
    return cmd.Start()
}

// Kullanım
openBrowser("https://ratlog.example.com/cli-login")
```

---

## 2. Token Alma

Kullanıcıdan token'ı almak için CLI'nizde bir input prompt kullanın.

### Rust (örnek: `dialoguer`)

```rust
use dialoguer::Input;

fn get_token_from_user() -> Result<String, Box<dyn std::error::Error>> {
    println!("Tarayıcıda giriş yapın ve token'ı kopyalayın.");
    println!("Token'ı buraya yapıştırın:");
    
    let token: String = Input::new()
        .with_prompt("Token")
        .interact_text()?;
    
    Ok(token.trim().to_string())
}
```

### Python

```python
def get_token_from_user() -> str:
    print("Tarayıcıda giriş yapın ve token'ı kopyalayın.")
    print("Token'ı buraya yapıştırın:")
    token = input("Token: ").strip()
    return token
```

### Go

```go
import (
    "bufio"
    "fmt"
    "os"
    "strings"
)

func getTokenFromUser() (string, error) {
    fmt.Println("Tarayıcıda giriş yapın ve token'ı kopyalayın.")
    fmt.Print("Token'ı buraya yapıştırın: ")
    
    reader := bufio.NewReader(os.Stdin)
    token, err := reader.ReadString('\n')
    if err != nil {
        return "", err
    }
    
    return strings.TrimSpace(token), nil
}
```

---

## 3. Token Doğrulama

Token'ı Ratlog Web API'sine göndererek doğrulayın.

### API Endpoint

```
GET {APP_URL}/api/user
Authorization: Bearer {token}
```

### Başarılı Yanıt (200 OK)

```json
{
  "id": 1,
  "name": "John Doe",
  "email": "john@example.com",
  "email_verified_at": "2026-02-16T10:00:00.000000Z",
  "created_at": "2026-02-15T08:00:00.000000Z",
  "updated_at": "2026-02-16T10:00:00.000000Z"
}
```

### Hata Yanıtları

- **401 Unauthorized**: Token geçersiz veya süresi dolmuş
- **500 Internal Server Error**: Sunucu hatası

### Rust Örneği (reqwest)

```rust
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;

async fn verify_token(base_url: &str, token: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/user", base_url);
    
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    
    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await?;
    
    if response.status().is_success() {
        let user: Value = response.json().await?;
        Ok(user)
    } else {
        Err(format!("Token doğrulama başarısız: {}", response.status()).into())
    }
}
```

### Python Örneği (requests)

```python
import requests

def verify_token(base_url: str, token: str) -> dict:
    url = f"{base_url}/api/user"
    headers = {"Authorization": f"Bearer {token}"}
    
    response = requests.get(url, headers=headers)
    response.raise_for_status()
    
    return response.json()
```

### Go Örneği

```go
import (
    "encoding/json"
    "fmt"
    "io"
    "net/http"
)

type User struct {
    ID    int    `json:"id"`
    Name  string `json:"name"`
    Email string `json:"email"`
}

func verifyToken(baseURL, token string) (*User, error) {
    url := fmt.Sprintf("%s/api/user", baseURL)
    
    req, err := http.NewRequest("GET", url, nil)
    if err != nil {
        return nil, err
    }
    
    req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", token))
    
    client := &http.Client{}
    resp, err := client.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    
    if resp.StatusCode != http.StatusOK {
        return nil, fmt.Errorf("token doğrulama başarısız: %d", resp.StatusCode)
    }
    
    body, err := io.ReadAll(resp.Body)
    if err != nil {
        return nil, err
    }
    
    var user User
    if err := json.Unmarshal(body, &user); err != nil {
        return nil, err
    }
    
    return &user, nil
}
```

---

## 4. Token Saklama

Token'ı güvenli bir şekilde yerel dosyaya kaydedin.

### Önerilen Konumlar

- **macOS/Linux**: `~/.config/ratlog/token` veya `~/.ratlog/token`
- **Windows**: `%APPDATA%\ratlog\token` veya `%USERPROFILE%\.ratlog\token`

### Rust Örneği

```rust
use std::fs;
use std::path::PathBuf;
use dirs;

fn get_token_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap())
        .join("ratlog");
    
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("token")
}

fn save_token(token: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_token_path();
    fs::write(&path, token)?;
    
    // Dosya izinlerini sınırla (sadece kullanıcı okuyabilsin)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    
    Ok(())
}

fn load_token() -> Result<String, Box<dyn std::error::Error>> {
    let path = get_token_path();
    Ok(fs::read_to_string(path)?.trim().to_string())
}
```

### Python Örneği

```python
import os
from pathlib import Path

def get_token_path() -> Path:
    config_dir = Path.home() / ".config" / "ratlog"
    config_dir.mkdir(parents=True, exist_ok=True)
    return config_dir / "token"

def save_token(token: str):
    path = get_token_path()
    path.write_text(token)
    # Dosya izinlerini sınırla
    os.chmod(path, 0o600)

def load_token() -> str:
    path = get_token_path()
    return path.read_text().strip()
```

### Go Örneği

```go
import (
    "os"
    "path/filepath"
    "io/ioutil"
)

func getTokenPath() (string, error) {
    homeDir, err := os.UserHomeDir()
    if err != nil {
        return "", err
    }
    
    configDir := filepath.Join(homeDir, ".config", "ratlog")
    os.MkdirAll(configDir, 0700)
    
    return filepath.Join(configDir, "token"), nil
}

func saveToken(token string) error {
    path, err := getTokenPath()
    if err != nil {
        return err
    }
    
    return ioutil.WriteFile(path, []byte(token), 0600)
}

func loadToken() (string, error) {
    path, err := getTokenPath()
    if err != nil {
        return "", err
    }
    
    data, err := ioutil.ReadFile(path)
    if err != nil {
        return "", err
    }
    
    return strings.TrimSpace(string(data)), nil
}
```

---

## 5. Tam Entegrasyon Örneği (Rust)

```rust
use std::process::Command;
use std::io::{self, Write};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use dirs;

const APP_URL: &str = "https://ratlog.example.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Ratlog CLI Login");
    println!();
    
    // Tarayıcıyı aç
    let url = format!("{}/cli-login", APP_URL);
    open_browser(&url)?;
    
    println!("Tarayıcı açıldı. Giriş yapın ve token'ı kopyalayın.");
    println!();
    
    // Token al
    print!("Token'ı yapıştırın: ");
    io::stdout().flush()?;
    
    let mut token = String::new();
    io::stdin().read_line(&mut token)?;
    let token = token.trim();
    
    // Token'ı doğrula
    println!("Token doğrulanıyor...");
    let user = verify_token(APP_URL, token).await?;
    
    // Token'ı kaydet
    save_token(token)?;
    
    println!();
    println!("✓ Giriş başarılı: {}", user["email"].as_str().unwrap());
    println!("Token kaydedildi.");
    
    Ok(())
}

fn open_browser(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    Command::new("open").arg(url).spawn()?;
    
    #[cfg(target_os = "windows")]
    Command::new("cmd").args(["/C", "start", url]).spawn()?;
    
    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(url).spawn()?;
    
    Ok(())
}

async fn verify_token(base_url: &str, token: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/user", base_url);
    
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    
    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await?;
    
    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        Err(format!("Token geçersiz: {}", response.status()).into())
    }
}

fn save_token(token: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap())
        .join("ratlog");
    
    fs::create_dir_all(&config_dir)?;
    let token_path = config_dir.join("token");
    fs::write(&token_path, token)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&token_path, fs::Permissions::from_mode(0o600))?;
    }
    
    Ok(())
}
```

---

## 6. API İsteklerinde Token Kullanımı

Token'ı kaydettikten sonra, API isteklerinde kullanın.

### Örnek: Log Paylaşma

```rust
async fn share_log(token: &str, content: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/logs", APP_URL);
    
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    
    let body = serde_json::json!({
        "content": content
    });
    
    let response = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await?;
    
    if response.status().is_success() {
        // Redirect response'dan URL'i al (Laravel redirect döner)
        let location = response.headers().get("location")
            .ok_or("Redirect location bulunamadı")?
            .to_str()?;
        
        Ok(location.to_string())
    } else {
        Err(format!("Log paylaşımı başarısız: {}", response.status()).into())
    }
}
```

---

## 7. Hata Yönetimi

### Yaygın Hatalar ve Çözümleri

1. **Token geçersiz (401)**
   - Kullanıcıya yeniden `ratlog login` çalıştırmasını söyleyin
   - Eski token'ı silin ve yeni token alın

2. **Ağ hatası**
   - `APP_URL`'nin doğru olduğundan emin olun
   - İnternet bağlantısını kontrol edin

3. **Token dosyası bulunamadı**
   - İlk login'de token dosyası oluşturulur
   - Dosya izinlerini kontrol edin

### Örnek Hata Yönetimi

```rust
fn handle_login_error(error: &dyn std::error::Error) {
    eprintln!("Giriş hatası: {}", error);
    eprintln!();
    eprintln!("Çözüm:");
    eprintln!("  1. Token'ın doğru kopyalandığından emin olun");
    eprintln!("  2. Tarayıcıda giriş yaptığınızdan emin olun");
    eprintln!("  3. Yeniden deneyin: ratlog login");
}
```

---

## 8. Test Etme

### Geliştirme Ortamı

1. Ratlog Web'i yerel olarak çalıştırın: `php artisan serve`
2. `APP_URL`'yi `http://localhost:8000` olarak ayarlayın
3. CLI uygulamanızı test edin

### Üretim Ortamı

1. Ratlog Web'i deploy edin
2. `APP_URL`'yi production URL'inize ayarlayın
3. CLI uygulamanızı kullanıcılara dağıtın

---

## 9. Güvenlik Notları

- Token'ı **asla** versiyon kontrolüne (git) eklemeyin
- Token dosyasını **sadece kullanıcı** okuyabilsin (chmod 600)
- Token'ı **HTTPS** üzerinden gönderin
- Token'ın **süresi dolduğunda** yeniden login isteyin
- Token'ı **güvenli bir yerde** saklayın (ör. `~/.config/ratlog/token`)

---

## 10. Ek Kaynaklar

- **Ratlog Web API**: `{APP_URL}/api/user` (GET, Bearer token gerekli)
- **CLI Login Sayfası**: `{APP_URL}/cli-login` (GET, auth gerekli)
- **Laravel Sanctum Dokümantasyonu**: https://laravel.com/docs/sanctum

---

## Sorular?

Entegrasyon sırasında sorun yaşarsanız:
1. Token'ın doğru kopyalandığından emin olun (boşluk, yeni satır yok)
2. `APP_URL`'nin doğru olduğunu kontrol edin
3. API endpoint'lerinin erişilebilir olduğunu doğrulayın
4. Tarayıcı konsolunda (F12) hataları kontrol edin
