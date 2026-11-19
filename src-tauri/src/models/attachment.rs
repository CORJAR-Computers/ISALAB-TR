use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Contenido de un archivo almacenado como BLOB en Firebird.
///
/// El backend lee el BLOB y lo entrega al webview como `data:` URL para que un
/// `<img>` lo muestre sin rutas locales (los tres PCs ven el mismo archivo).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentContent {
    pub file_name: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    /// `data:<mime>;base64,<...>` listo para usar en el webview.
    pub data_url: String,
}

impl AttachmentContent {
    /// Construye la respuesta a partir del BLOB y sus metadatos.
    pub fn new(
        file_name: String,
        mime_type: Option<String>,
        content: Vec<u8>,
    ) -> Self {
        let mime = mime_type
            .clone()
            .or_else(|| mime_for_file_name(&file_name))
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let size_bytes = content.len() as i64;
        let data_url = format!("data:{mime};base64,{}", STANDARD.encode(&content));
        Self {
            file_name,
            mime_type,
            size_bytes,
            data_url,
        }
    }
}

/// Deduce el MIME a partir de la extensión cuando la fila no lo guardó.
pub fn mime_for_file_name(file_name: &str) -> Option<String> {
    let ext = std::path::Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    Some(
        match ext.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "pdf" => "application/pdf",
            "p12" | "pfx" => "application/x-pkcs12",
            _ => return None,
        }
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_data_url_and_infers_mime() {
        let content = AttachmentContent::new("foto.PNG".into(), None, vec![1, 2, 3]);
        assert_eq!(content.mime_type, None);
        assert_eq!(content.size_bytes, 3);
        assert!(content.data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn keeps_an_explicit_mime() {
        let content = AttachmentContent::new(
            "archivo.bin".into(),
            Some("application/pdf".into()),
            vec![9],
        );
        assert!(content.data_url.starts_with("data:application/pdf;base64,"));
    }

    #[test]
    fn unknown_extension_falls_back_to_octet_stream() {
        let content = AttachmentContent::new("x.xyz".into(), None, vec![]);
        assert!(content
            .data_url
            .starts_with("data:application/octet-stream;base64,"));
    }
}
