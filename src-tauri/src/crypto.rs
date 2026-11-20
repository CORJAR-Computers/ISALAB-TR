//! Cifrado de secretos.
//!
//! Hay dos ámbitos y dos formatos distintos:
//!
//! - **Local** (`enc:v1:`): **DPAPI de Windows** (`CryptProtectData`). El
//!   cifrado queda ligado al usuario de Windows del equipo, así que solo sirve
//!   para valores de esa PC. Se usa para la contraseña de conexión guardada en
//!   `db-connection.json`: ese archivo debe leerse *antes* de abrir la base y
//!   no puede depender de ella.
//! - **Compartido** (`enc:v2:`): **ChaCha20-Poly1305** con una clave de 32 bytes
//!   derivada (PBKDF2-HMAC-SHA256) de la contraseña de conexión a Firebird más
//!   un salt aleatorio guardado en `CLINIC_SETTINGS`. Los tres PCs de la clínica
//!   comparten usuario y contraseña de base, así que derivan la misma clave y
//!   descifran los mismos secretos (`ai.groq_api_key`, `smtp.password`). Una
//!   copia del `.fdb` por sí sola no permite leerlos.
//!
//! Los valores legacy sin prefijo (texto plano de versiones previas) se
//! detectan y se re-cifran en el primer `get()`. Los `enc:v1:` que aún existan
//! en la base se siguen leyendo con DPAPI y se migran a `enc:v2:` al guardar.
//!
//! En plataformas no-Windows (builds de desarrollo macOS/Linux) el cifrado
//! local es un passthrough explícitamente documentado: el producto de
//! producción es Windows (instalador NSIS). El cifrado compartido (AEAD) sí es
//! real en todas las plataformas.

use std::sync::{Mutex, RwLock};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;

use crate::error::AppError;

/// Prefijo de los valores cifrados con DPAPI (ámbito local).
const LOCAL_PREFIX: &str = "enc:v1:";
/// Prefijo de los valores cifrados con AEAD (ámbito compartido de la clínica).
const SHARED_PREFIX: &str = "enc:v2:";
/// Iteraciones de PBKDF2 para derivar la clave compartida.
const KDF_ITERATIONS: u32 = 100_000;
/// Longitud del salt (bytes) que se guarda en `CLINIC_SETTINGS`.
pub const SALT_LEN: usize = 16;
/// Contraseña de desarrollo cuando la app corre en embedded (sin contraseña de
/// conexión). En producción siempre hay una contraseña de usuario de Firebird.
const DEV_PASSWORD: &str = "isalab-dev-no-db-password";

/// Contraseña de conexión con la que se deriva la clave compartida. Se fija al
/// arrancar (`AppState::init`) a partir de la configuración resuelta.
static SHARED_PASSWORD: RwLock<Option<String>> = RwLock::new(None);
/// Última clave derivada, cacheada por salt (PBKDF2 es costoso y `get()` de
/// ajustes se llama con frecuencia).
static SHARED_KEY_CACHE: Mutex<Option<([u8; SALT_LEN], [u8; 32])>> = Mutex::new(None);

/// Indica si `stored` está cifrado con cualquiera de los dos formatos.
pub fn is_encrypted(stored: &str) -> bool {
    stored.starts_with(LOCAL_PREFIX) || stored.starts_with(SHARED_PREFIX)
}

/// `true` si el valor usa el formato compartido (AEAD).
pub fn is_shared_encrypted(stored: &str) -> bool {
    stored.starts_with(SHARED_PREFIX)
}

// ====================== Ámbito local (DPAPI) ================================

/// `true` si el valor usa el formato local (DPAPI).
pub fn is_local_encrypted(stored: &str) -> bool {
    stored.starts_with(LOCAL_PREFIX)
}

/// Cifra `plaintext` con DPAPI y devuelve `enc:v1:<base64>`.
pub fn encrypt_local(plaintext: &str) -> Result<String, AppError> {
    let blob = imp::encrypt(plaintext)?;
    Ok(format!("{LOCAL_PREFIX}{}", STANDARD.encode(blob)))
}

/// Descifra un valor `enc:v1:<base64>` con DPAPI.
///
/// `Ok(None)` si no tiene el prefijo local (no es un valor de este formato).
pub fn decrypt_local(stored: &str) -> Result<Option<String>, AppError> {
    let Some(b64) = stored.strip_prefix(LOCAL_PREFIX) else {
        return Ok(None);
    };
    let blob = STANDARD
        .decode(b64)
        .map_err(|_| AppError::Internal("Valor local corrupto (base64 inválido)".into()))?;
    imp::decrypt(&blob).map(Some)
}

// ==================== Ámbito compartido (AEAD) ==============================

/// Fija la contraseña de conexión usada para derivar la clave compartida.
/// Debe llamarse una vez al arrancar, antes de leer/escribir ajustes.
pub fn set_shared_password(password: &str) {
    let password = if password.is_empty() {
        DEV_PASSWORD.to_string()
    } else {
        password.to_string()
    };
    if let Ok(mut guard) = SHARED_PASSWORD.write() {
        *guard = Some(password);
    }
    if let Ok(mut cache) = SHARED_KEY_CACHE.lock() {
        *cache = None;
    }
}

/// Genera un salt aleatorio para la clave compartida.
pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Codifica bytes como hexadecimal en minúsculas (para guardar el salt).
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decodifica una cadena hexadecimal; `None` si es impar o inválida.
pub fn hex_decode(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if text.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    for pair in text.as_bytes().chunks_exact(2) {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        out.push((high * 16 + low) as u8);
    }
    Some(out)
}

/// Deriva (y cachea) la clave AEAD a partir de la contraseña y el salt.
fn derive_shared_key(salt: &[u8]) -> [u8; 32] {
    let cached = SHARED_KEY_CACHE.lock().map(|g| *g).unwrap_or(None);
    if let Some((cached_salt, key)) = cached {
        if cached_salt.as_slice() == salt {
            return key;
        }
    }

    let password = SHARED_PASSWORD
        .read()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(|| DEV_PASSWORD.to_string());

    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, KDF_ITERATIONS, &mut key);

    if salt.len() == SALT_LEN {
        let mut copy = [0u8; SALT_LEN];
        copy.copy_from_slice(salt);
        if let Ok(mut cache) = SHARED_KEY_CACHE.lock() {
            *cache = Some((copy, key));
        }
    }
    key
}

/// Cifra `plaintext` con la clave compartida y devuelve `enc:v2:<base64>`.
/// `salt` es el guardado en `CLINIC_SETTINGS` (ver `repositories::settings`).
pub fn encrypt_shared(plaintext: &str, salt: &[u8]) -> Result<String, AppError> {
    let key = derive_shared_key(salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| AppError::Internal("Clave de cifrado inválida".into()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| AppError::Internal("No se pudo cifrar el secreto compartido".into()))?;

    let mut packed = Vec::with_capacity(12 + ciphertext.len());
    packed.extend_from_slice(&nonce_bytes);
    packed.extend_from_slice(&ciphertext);
    Ok(format!("{SHARED_PREFIX}{}", STANDARD.encode(packed)))
}

/// Descifra un valor guardado en la base.
///
/// - `enc:v2:` → AEAD compartido.
/// - `enc:v1:` → DPAPI local (valor de una versión previa; se migra al guardar).
/// - Sin prefijo → `Ok(None)` (texto plano legacy).
pub fn decrypt_shared(stored: &str, salt: &[u8]) -> Result<Option<String>, AppError> {
    if is_local_encrypted(stored) {
        return decrypt_local(stored);
    }
    let Some(b64) = stored.strip_prefix(SHARED_PREFIX) else {
        return Ok(None);
    };
    let packed = STANDARD
        .decode(b64)
        .map_err(|_| AppError::Internal("Secreto almacenado corrupto (base64 inválido)".into()))?;
    if packed.len() < 12 {
        return Err(AppError::Internal(
            "Secreto almacenado corrupto (longitud inválida)".into(),
        ));
    }

    let key = derive_shared_key(salt);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| AppError::Internal("Clave de cifrado inválida".into()))?;
    let (nonce_bytes, ciphertext) = packed.split_at(12);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| {
            AppError::Internal(
                "No se pudo descifrar el secreto compartido: ¿cambió la contraseña de conexión a la base? Vuelve a guardar la clave en Ajustes.".into(),
            )
        })?;
    String::from_utf8(plaintext)
        .map(Some)
        .map_err(|_| AppError::Internal("El secreto descifrado no es texto UTF-8 válido".into()))
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    /// Cifra `plaintext` con DPAPI en el ámbito del usuario actual.
    pub fn encrypt(plaintext: &str) -> Result<Vec<u8>, AppError> {
        let bytes = plaintext.as_bytes();
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        // SAFETY: entrada apunta a bytes válidos; DPAPI asigna el buffer de
        // salida y nosotros lo liberamos con LocalFree.
        let ok = unsafe {
            CryptProtectData(
                &in_blob,
                std::ptr::null::<u16>(),                // sin descripción
                std::ptr::null::<CRYPT_INTEGER_BLOB>(), // sin entropy adicional
                std::ptr::null(),                       // reservado
                std::ptr::null(),                       // sin prompt
                CRYPTPROTECT_UI_FORBIDDEN,              // nunca mostrar diálogos
                &mut out_blob,
            )
        };
        if ok == 0 {
            return Err(AppError::Internal(
                "No se pudo cifrar el valor con DPAPI (Windows)".into(),
            ));
        }
        // SAFETY: out_blob.pbData apunta a cbData bytes asignados por la API.
        let result =
            unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }
                .to_vec();
        // SAFETY: LocalFree libera el buffer devuelto por CryptProtectData.
        unsafe { LocalFree(out_blob.pbData as *mut _) };
        Ok(result)
    }

    /// Descifra un blob DPAPI del usuario actual.
    pub fn decrypt(blob: &[u8]) -> Result<String, AppError> {
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: blob.len() as u32,
            pbData: blob.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        // SAFETY: misma gestión de buffers que encrypt (entrada válida, salida
        // asignada por DPAPI y liberada con LocalFree).
        let ok = unsafe {
            CryptUnprotectData(
                &in_blob,
                std::ptr::null_mut(), // ppszDataDescr: *mut *mut u16 (no usado)
                std::ptr::null::<CRYPT_INTEGER_BLOB>(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };
        if ok == 0 {
            return Err(AppError::Internal(
                "No se pudo descifrar el valor local: ¿se movió el archivo a otro usuario o equipo?".into(),
            ));
        }
        // SAFETY: mismo contrato de buffers que encrypt.
        let result =
            unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }
                .to_vec();
        unsafe { LocalFree(out_blob.pbData as *mut _) };
        String::from_utf8(result)
            .map_err(|_| AppError::Internal("El valor descifrado no es texto UTF-8 válido".into()))
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    /// Passthrough para builds de desarrollo no-Windows (macOS/Linux).
    /// No cifra: la app de producción es Windows y usa DPAPI (módulo `imp`
    /// anterior). El valor se guarda como base64 del texto plano bajo el
    /// prefijo `enc:v1:` (NO es cifrado real — no distribuir builds
    /// no-Windows a clínicas; el pipeline oficial solo compila Windows).
    pub fn encrypt(plaintext: &str) -> Result<Vec<u8>, AppError> {
        Ok(plaintext.as_bytes().to_vec())
    }

    pub fn decrypt(blob: &[u8]) -> Result<String, AppError> {
        String::from_utf8(blob.to_vec())
            .map_err(|_| AppError::Internal("El valor descifrado no es texto UTF-8 válido".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "gsk_live_1234567890abcdef";
    const SALT: [u8; SALT_LEN] = [7u8; SALT_LEN];

    #[test]
    fn local_roundtrip_encrypt_decrypt() {
        let stored = encrypt_local(SECRET).unwrap();
        assert!(is_local_encrypted(&stored));
        assert!(is_encrypted(&stored));
        // El texto plano nunca aparece en el valor persistido.
        assert!(!stored.contains(SECRET));
        assert_eq!(decrypt_local(&stored).unwrap().as_deref(), Some(SECRET));
    }

    #[test]
    fn shared_roundtrip_encrypt_decrypt() {
        let stored = encrypt_shared(SECRET, &SALT).unwrap();
        assert!(is_shared_encrypted(&stored));
        assert!(is_encrypted(&stored));
        assert!(!stored.contains(SECRET));
        assert_eq!(
            decrypt_shared(&stored, &SALT).unwrap().as_deref(),
            Some(SECRET)
        );
    }

    #[test]
    fn shared_ciphertext_is_randomized() {
        // Dos cifrados del mismo valor deben diferir (nonce aleatorio).
        let a = encrypt_shared(SECRET, &SALT).unwrap();
        let b = encrypt_shared(SECRET, &SALT).unwrap();
        assert_ne!(a, b);
        assert_eq!(decrypt_shared(&a, &SALT).unwrap().as_deref(), Some(SECRET));
        assert_eq!(decrypt_shared(&b, &SALT).unwrap().as_deref(), Some(SECRET));
    }

    #[test]
    fn empty_secret_roundtrip() {
        let stored = encrypt_shared("", &SALT).unwrap();
        assert!(is_shared_encrypted(&stored));
        assert_eq!(decrypt_shared(&stored, &SALT).unwrap().as_deref(), Some(""));
    }

    #[test]
    fn legacy_plaintext_returns_none() {
        assert!(!is_encrypted("gsk_plaintext_legacy"));
        assert_eq!(decrypt_shared("gsk_plaintext_legacy", &SALT).unwrap(), None);
    }

    #[test]
    fn legacy_dpapi_value_is_read_by_shared_decrypt() {
        let stored = encrypt_local(SECRET).unwrap();
        assert_eq!(
            decrypt_shared(&stored, &SALT).unwrap().as_deref(),
            Some(SECRET)
        );
    }

    #[test]
    fn corrupt_stored_value_fails() {
        let err = decrypt_shared("enc:v2:###no-es-base64###", &SALT).unwrap_err();
        assert!(matches!(err, AppError::Internal(_)));
        let err = decrypt_shared("enc:v2:AAAA", &SALT).unwrap_err();
        assert!(matches!(err, AppError::Internal(_)));
    }

    #[test]
    fn a_different_salt_cannot_decrypt() {
        let stored = encrypt_shared(SECRET, &SALT).unwrap();
        let err = decrypt_shared(&stored, &[9u8; SALT_LEN]).unwrap_err();
        assert!(matches!(err, AppError::Internal(_)));
    }

    #[test]
    fn hex_helpers_roundtrip() {
        let bytes = [0u8, 1, 15, 16, 254, 255];
        let text = hex_encode(&bytes);
        assert_eq!(text, "00010f10feff");
        assert_eq!(hex_decode(&text).unwrap(), bytes);
        assert!(hex_decode("abc").is_none());
        assert!(hex_decode("zz").is_none());
    }
}
