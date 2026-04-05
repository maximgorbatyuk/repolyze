use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

const SETTINGS_DIR: &str = ".repolyze";
const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Deserialize)]
struct SettingsFile {
    #[serde(default)]
    users: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// Forward map: display name -> list of emails (lowercased)
    name_to_emails: HashMap<String, Vec<String>>,
    /// Reverse map: lowercased email -> display name
    email_to_name: HashMap<String, String>,
}

impl Settings {
    /// Ensure `.repolyze/settings.json` exists in `dir`, then load it.
    /// If the file does not exist, creates it with `{}`.
    /// Returns `Settings::default()` on any I/O or parse error (non-fatal).
    pub fn ensure_and_load(dir: &Path) -> Self {
        let settings_dir = dir.join(SETTINGS_DIR);
        let settings_file = settings_dir.join(SETTINGS_FILE);

        if !settings_file.exists() {
            let _ = fs::create_dir_all(&settings_dir);
            let _ = fs::write(&settings_file, "{}");
        }

        Self::load_from(&settings_file)
    }

    fn load_from(path: &Path) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let file: SettingsFile = match serde_json::from_str(&content) {
            Ok(f) => f,
            Err(_) => return Self::default(),
        };

        Self::from_users(file.users)
    }

    fn from_users(users: HashMap<String, Vec<String>>) -> Self {
        let mut name_to_emails: HashMap<String, Vec<String>> = HashMap::new();
        let mut email_to_name: HashMap<String, String> = HashMap::new();

        for (name, emails) in users {
            let lowered: Vec<String> = emails.iter().map(|e| e.to_lowercase()).collect();
            for email in &lowered {
                if let Some(prev) = email_to_name.insert(email.clone(), name.clone())
                    && prev != name
                {
                    eprintln!(
                        "warning: email '{email}' appears under both '{prev}' and '{name}' in settings; using '{name}'"
                    );
                }
            }
            name_to_emails.insert(name, lowered);
        }

        Self {
            name_to_emails,
            email_to_name,
        }
    }

    /// Returns the canonical display key for a given email.
    /// If the email is mapped to a user name in settings, returns that name.
    /// Otherwise returns the email (lowercased).
    pub fn canonical_key(&self, email: &str) -> String {
        let lower = email.to_lowercase();
        self.email_to_name.get(&lower).cloned().unwrap_or(lower)
    }

    /// Returns all lowercased emails associated with a canonical key.
    /// If the key is a configured user name, returns all their emails.
    /// Otherwise returns a single-element vec with the key itself (treated as email).
    pub fn emails_for_key(&self, key: &str) -> Vec<String> {
        if let Some(emails) = self.name_to_emails.get(key) {
            emails.clone()
        } else {
            vec![key.to_lowercase()]
        }
    }

    pub fn has_aliases(&self) -> bool {
        !self.email_to_name.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_has_no_aliases() {
        let s = Settings::default();
        assert!(!s.has_aliases());
        assert_eq!(s.canonical_key("alice@example.com"), "alice@example.com");
    }

    #[test]
    fn canonical_key_returns_name_when_mapped() {
        let users = HashMap::from([(
            "Alice".to_string(),
            vec!["alice@work.com".to_string(), "alice@home.com".to_string()],
        )]);
        let s = Settings::from_users(users);

        assert_eq!(s.canonical_key("alice@work.com"), "Alice");
        assert_eq!(s.canonical_key("alice@home.com"), "Alice");
        assert_eq!(s.canonical_key("ALICE@WORK.COM"), "Alice");
    }

    #[test]
    fn canonical_key_returns_email_when_not_mapped() {
        let users = HashMap::from([("Alice".to_string(), vec!["alice@work.com".to_string()])]);
        let s = Settings::from_users(users);

        assert_eq!(s.canonical_key("bob@example.com"), "bob@example.com");
    }

    #[test]
    fn emails_for_key_returns_all_emails_for_user() {
        let users = HashMap::from([(
            "Alice".to_string(),
            vec!["alice@work.com".to_string(), "alice@home.com".to_string()],
        )]);
        let s = Settings::from_users(users);

        let mut emails = s.emails_for_key("Alice");
        emails.sort();
        assert_eq!(emails, vec!["alice@home.com", "alice@work.com"]);
    }

    #[test]
    fn emails_for_key_returns_key_as_email_when_not_a_user() {
        let s = Settings::default();
        assert_eq!(s.emails_for_key("bob@example.com"), vec!["bob@example.com"]);
    }

    #[test]
    fn ensure_and_load_creates_file_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let s = Settings::ensure_and_load(dir.path());

        assert!(!s.has_aliases());
        assert!(dir.path().join(".repolyze/settings.json").exists());

        let content = fs::read_to_string(dir.path().join(".repolyze/settings.json")).unwrap();
        assert_eq!(content, "{}");
    }

    #[test]
    fn ensure_and_load_reads_existing_config() {
        let dir = tempfile::tempdir().unwrap();
        let settings_dir = dir.path().join(".repolyze");
        fs::create_dir_all(&settings_dir).unwrap();
        fs::write(
            settings_dir.join("settings.json"),
            r#"{"users": {"Alice": ["alice@work.com", "alice@home.com"]}}"#,
        )
        .unwrap();

        let s = Settings::ensure_and_load(dir.path());

        assert!(s.has_aliases());
        assert_eq!(s.canonical_key("alice@work.com"), "Alice");
        assert_eq!(s.canonical_key("alice@home.com"), "Alice");
    }

    #[test]
    fn load_handles_malformed_json_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let settings_dir = dir.path().join(".repolyze");
        fs::create_dir_all(&settings_dir).unwrap();
        fs::write(settings_dir.join("settings.json"), "not json").unwrap();

        let s = Settings::ensure_and_load(dir.path());
        assert!(!s.has_aliases());
    }

    #[test]
    fn load_handles_empty_users_object() {
        let dir = tempfile::tempdir().unwrap();
        let settings_dir = dir.path().join(".repolyze");
        fs::create_dir_all(&settings_dir).unwrap();
        fs::write(settings_dir.join("settings.json"), r#"{"users": {}}"#).unwrap();

        let s = Settings::ensure_and_load(dir.path());
        assert!(!s.has_aliases());
    }
}
