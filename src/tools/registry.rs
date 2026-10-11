use serde_json::{json, Map, Value};
use winreg::{enums::*, RegKey, RegValue};

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "registry_read" => registry_read(args),
        _ => json!({"error": format!("Unknown registry tool: {}", name)}),
    }
}

fn registry_read(args: &Value) -> Value {
    let key_path = args
        .get("key")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let value_name = args.get("value_name").and_then(|v| v.as_str());
    let recursive = args
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if key_path.is_empty() {
        return json!({"error": "Missing 'key' parameter"});
    }

    let (key, normalized) = match open_allowed_key(key_path) {
        Ok(pair) => pair,
        Err(err) => return json!({"error": err, "key": key_path}),
    };

    snapshot_key(&key, &normalized, value_name, recursive)
}

fn open_allowed_key(key_path: &str) -> Result<(RegKey, String), String> {
    let normalized = key_path.trim().replace('/', "\\");
    let upper = normalized.to_uppercase();
    for blocked in [
        "HKLM\\SAM",
        "HKLM\\SECURITY",
        "HKEY_LOCAL_MACHINE\\SAM",
        "HKEY_LOCAL_MACHINE\\SECURITY",
    ] {
        if upper == blocked || upper.starts_with(&(blocked.to_string() + "\\")) {
            return Err("Access denied to protected registry hives".to_string());
        }
    }

    let (root, subkey) = if let Some(rest) = strip_prefix_ci(&normalized, "HKLM\\") {
        (RegKey::predef(HKEY_LOCAL_MACHINE), rest)
    } else if let Some(rest) = strip_prefix_ci(&normalized, "HKEY_LOCAL_MACHINE\\") {
        (RegKey::predef(HKEY_LOCAL_MACHINE), rest)
    } else if let Some(rest) = strip_prefix_ci(&normalized, "HKCU\\") {
        (RegKey::predef(HKEY_CURRENT_USER), rest)
    } else if let Some(rest) = strip_prefix_ci(&normalized, "HKEY_CURRENT_USER\\") {
        (RegKey::predef(HKEY_CURRENT_USER), rest)
    } else {
        return Err("Only HKLM and HKCU are supported".to_string());
    };

    let allowed = subkey.eq_ignore_ascii_case("SOFTWARE")
        || subkey.to_uppercase().starts_with("SOFTWARE\\")
        || subkey.eq_ignore_ascii_case("ENVIRONMENT")
        || subkey.to_uppercase().starts_with("ENVIRONMENT\\");
    let hkcu = upper.starts_with("HKCU\\") || upper.starts_with("HKEY_CURRENT_USER\\");
    let hklm = upper.starts_with("HKLM\\") || upper.starts_with("HKEY_LOCAL_MACHINE\\");
    if !(hklm
        && (subkey.eq_ignore_ascii_case("SOFTWARE")
            || subkey.to_uppercase().starts_with("SOFTWARE\\")))
        && !(hkcu && allowed)
    {
        return Err("Registry path outside whitelist: allow HKLM\\SOFTWARE, HKCU\\SOFTWARE, HKCU\\Environment".to_string());
    }

    root.open_subkey_with_flags(subkey, KEY_READ)
        .map(|key| (key, normalized))
        .map_err(|err| format!("Failed to open key: {}", err))
}

fn snapshot_key(key: &RegKey, key_path: &str, value_name: Option<&str>, recursive: bool) -> Value {
    let mut out = Map::new();
    out.insert("key".to_string(), json!(key_path));
    if let Some(name) = value_name {
        match key.get_raw_value(name) {
            Ok(value) => {
                out.insert("value_name".to_string(), json!(display_name(name)));
                out.insert("value".to_string(), format_value(&value));
            }
            Err(err) => {
                return json!({"key": key_path, "value_name": display_name(name), "error": err.to_string()})
            }
        }
    } else {
        out.insert("values".to_string(), read_values(key));
    }
    if recursive {
        let mut subkeys = Map::new();
        for item in key.enum_keys() {
            match item {
                Ok(name) => match key.open_subkey_with_flags(&name, KEY_READ) {
                    Ok(child) => {
                        let child_path = format!("{}\\{}", key_path, name);
                        subkeys.insert(name, snapshot_key(&child, &child_path, value_name, false));
                    }
                    Err(err) => {
                        subkeys.insert(name.clone(), json!({"key": format!("{}\\{}", key_path, name), "error": err.to_string()}));
                    }
                },
                Err(err) => {
                    subkeys.insert(format!("__error_{}", subkeys.len()), json!(err.to_string()));
                }
            }
        }
        out.insert("subkeys".to_string(), Value::Object(subkeys));
    }
    Value::Object(out)
}

fn read_values(key: &RegKey) -> Value {
    let mut values = Map::new();
    for item in key.enum_values() {
        match item {
            Ok((name, value)) => {
                values.insert(display_name(&name), format_value(&value));
            }
            Err(err) => {
                values.insert(format!("__error_{}", values.len()), json!(err.to_string()));
            }
        }
    }
    Value::Object(values)
}

fn format_value(value: &RegValue) -> Value {
    let data = match value.vtype {
        REG_DWORD if value.bytes.len() >= 4 => {
            json!(u32::from_le_bytes(value.bytes[..4].try_into().unwrap()))
        }
        REG_QWORD if value.bytes.len() >= 8 => {
            json!(u64::from_le_bytes(value.bytes[..8].try_into().unwrap()))
        }
        REG_MULTI_SZ => json!(decode_multi_sz(&value.bytes)),
        REG_SZ | REG_EXPAND_SZ => json!(decode_utf16(&value.bytes)),
        _ => json!(value.bytes),
    };
    json!({"type": format!("{:?}", value.vtype), "data": data})
}

fn strip_prefix_ci<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() >= prefix.len() && input[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&input[prefix.len()..])
    } else {
        None
    }
}

fn display_name(name: &str) -> String {
    if name.is_empty() {
        "(Default)".to_string()
    } else {
        name.to_string()
    }
}

fn decode_utf16(bytes: &[u8]) -> String {
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    let end = words
        .iter()
        .position(|word| *word == 0)
        .unwrap_or(words.len());
    String::from_utf16_lossy(&words[..end])
}

fn decode_multi_sz(bytes: &[u8]) -> Vec<String> {
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    words
        .split(|word| *word == 0)
        .filter(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}
