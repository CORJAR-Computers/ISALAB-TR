//! Cifrado de secretos en repositorio (`groq_api_key`).
//!
//! Estrategia: **DPAPI de Windows** (`CryptProtectData`). El cifrado queda
//! ligado a la sesión del usuario de Windows actual, de modo que un tercero
//! que copie la base de datos no pueda leer la clave sin la identidad del
//! usuario que la guardó. No requiere contraseña maestra ni UI adicional.
//!
//! Formato de almacenamiento en BD: `enc:v1:<base64(blob)>`.
//! Los valores legacy (texto plano de versiones previas) se detectan por la
//! ausencia del prefijo y se re-cifran automáticamente en el primer `get()`.
//!
//! En plataformas no-Windows (builds de desarrollo macOS/Linux) el "cifrado"
//! es un passthrough explícitamente documentado: el producto de producción
//! es Windows (instalador NSIS con Firebird Embedded).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::error::AppError;

/// Prefijo que identifica valores cifrados con este módulo en la BD.
const ENC_PREFIX: &str = "enc:v1:";

/// Indica si `stored` (valor leído de la BD) está cifrado con este módulo.
pub fn is_encrypted(stored: &str) -> bool {
    stored.starts_with(ENC_PREFIX)
}

/// Cifra `plaintext` y devuelve el valor listo para persistir en BD
/// (`enc:v1:<base64>`). En no-Windows devuelve base64 del texto plano.
pub fn encrypt_to_db(plaintext: &str) -> Result<String, AppError> {
    let blob = imp::encrypt(plaintext)?;
    Ok(format!("{ENC_PREFIX}{}", STANDARD.encode(blob)))
}

/// Descifra un valor leído de la BD con formato `enc:v1:<base64>`.
/// Devuelve `Ok(None)` si el valor no tiene el prefijo (legacy texto plano);
/// `Ok(Some(plaintext))` si se descifró; `Err` si el valor está corrupto o
/// no puede descifrarse (p. ej. la BD se movió a otro usuario de Windows).
pub fn decrypt_from_db(stored: &str) -> Result<Option<String>, AppError> {
    let Some(b64) = stored.strip_prefix(ENC_PREFIX) else {
        return Ok(None);
    };
    let blob = STANDARD
        .decode(b64)
        .map_err(|_| AppError::Internal("Secreto almacenado corrupto (base64 inválido)".into()))?;
    imp::decrypt(&blob).map(Some)
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
                "No se pudo cifrar el secreto con DPAPI (Windows)".into(),
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
                "No se pudo descifrar el secreto: ¿se movió la base de datos a otro usuario o máquina? Vuelve a configurar la clave de IA en Ajustes.".into(),
            ));
        }
        // SAFETY: mismo contrato de buffers que encrypt.
        let result =
            unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }
                .to_vec();
        unsafe { LocalFree(out_blob.pbData as *mut _) };
        String::from_utf8(result).map_err(|_| {
            AppError::Internal("El secreto descifrado no es texto UTF-8 válido".into())
        })
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
        String::from_utf8(blob.to_vec()).map_err(|_| {
            AppError::Internal("El secreto descifrado no es texto UTF-8 válido".into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "gsk_live_1234567890abcdef";

    #[test]
    fn test_roundtrip_encrypt_decrypt() {
        let stored = encrypt_to_db(SECRET).unwrap();
        assert!(is_encrypted(&stored));
        // El texto plano nunca aparece en el valor persistido.
        assert!(!stored.contains(SECRET));
        assert_eq!(decrypt_from_db(&stored).unwrap().as_deref(), Some(SECRET));
    }

    #[test]
    fn test_empty_secret_roundtrip() {
        // El repositorio nunca cifra vacíos, pero el módulo debe ser robusto.
        let stored = encrypt_to_db("").unwrap();
        assert!(is_encrypted(&stored));
        assert_eq!(decrypt_from_db(&stored).unwrap().as_deref(), Some(""));
    }

    #[test]
    fn test_legacy_plaintext_returns_none() {
        // Valor sin prefijo = instalación previa con texto plano.
        assert!(!is_encrypted("gsk_plaintext_legacy"));
        assert_eq!(decrypt_from_db("gsk_plaintext_legacy").unwrap(), None);
    }

    #[test]
    fn test_corrupt_stored_value_fails() {
        let err = decrypt_from_db("enc:v1:###no-es-base64###").unwrap_err();
        assert!(matches!(err, AppError::Internal(_)));
    }
}
