//! Log rotativo en `app_data/logs` para diagnóstico local.
//!
//! Por qué existe: en la app empaquetada el `console.error` del webview no deja
//! rastro y Rust no escribía nada en disco, así que un fallo en la clínica
//! (Firebird caído, un comando que revienta, la UI que no pinta datos) no se
//! podía reconstruir después. Este módulo es el sumidero común de backend y
//! frontend.
//!
//! Decisiones:
//! - Sin dependencias nuevas: apertura en modo append por línea (el volumen es
//!   bajo: arranque, errores y lo que reenvía la UI).
//! - Abrir/cerrar en cada escritura evita mantener un handle abierto, que en
//!   Windows impediría renombrar el archivo al rotar.
//! - Rotación por tamaño: `isalab.log` + `isalab.1.log` … `isalab.5.log`.
//! - Un mensaje nunca parte el log en varias líneas: los saltos se aplanan.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Tamaño máximo del archivo actual antes de rotar (2 MB).
const MAX_BYTES: u64 = 2 * 1024 * 1024;
/// Copias rotadas que se conservan (`isalab.1.log` … `isalab.5.log`).
const MAX_ROTATED: usize = 5;
const LOG_NAME: &str = "isalab.log";

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
        }
    }

    /// Nivel a partir del texto que envía el frontend.
    pub fn parse(value: &str) -> Level {
        match value.trim().to_ascii_lowercase().as_str() {
            "error" => Level::Error,
            "warn" | "warning" => Level::Warn,
            _ => Level::Info,
        }
    }
}

/// Prepara `<app_data>/logs`, recuerda la ruta y escribe la línea de apertura.
///
/// Idempotente: una segunda llamada no cambia el directorio ya fijado. Devuelve
/// la ruta usada (o `None` si no se pudo crear), para poder mostrarla en el
/// diagnóstico.
pub fn init(app_data: &Path) -> Option<PathBuf> {
    let logs = app_data.join("logs");
    if fs::create_dir_all(&logs).is_err() {
        return None;
    }
    let _ = LOG_DIR.set(logs.clone());
    info("app", &format!("log iniciado en {}", logs.display()));
    Some(logs)
}

pub fn dir() -> Option<&'static Path> {
    LOG_DIR.get().map(PathBuf::as_path)
}

pub fn info(target: &str, message: &str) {
    write(Level::Info, target, message);
}

pub fn warn(target: &str, message: &str) {
    write(Level::Warn, target, message);
}

pub fn error(target: &str, message: &str) {
    write(Level::Error, target, message);
}

/// Registra un error/aviso reenviado por la UI (comando `log_frontend`).
///
/// `context` es opcional y descriptivo (queryKey, vista, comando IPC…). No se
/// filtra contenido clínico aquí: el llamador decide qué manda; este log vive
/// en el equipo del usuario.
pub fn frontend(level: &str, message: &str, context: Option<&str>) {
    let full = match context {
        Some(ctx) if !ctx.trim().is_empty() => format!("{message} | {ctx}"),
        _ => message.to_string(),
    };
    write(Level::parse(level), "frontend", &full);
}

pub fn write(level: Level, target: &str, message: &str) {
    if let Some(dir) = dir() {
        append(dir, level, target, message, MAX_BYTES, MAX_ROTATED);
    }
    // En desarrollo también a stderr; en release solo el archivo.
    if cfg!(debug_assertions) {
        eprintln!("[{}] [{}] {}", level.label(), target, message);
    }
}

/// Contenido de los log actuales, del más reciente al más antiguo, acotado a
/// `limit_bytes` en total (el diagnóstico viaja en un correo, no en un backup).
pub fn read_logs(limit_bytes: usize) -> Vec<(String, String)> {
    let Some(dir) = dir() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy().to_string();
            (name.starts_with("isalab") && name.ends_with(".log"))
                .then(|| (name.clone(), path.clone()))
        })
        .collect();
    // isalab.log primero (lo más reciente), luego .1, .2…
    files.sort_by_key(|(name, _)| {
        if name == LOG_NAME {
            0
        } else {
            name.trim_start_matches("isalab.")
                .trim_end_matches(".log")
                .parse::<usize>()
                .unwrap_or(usize::MAX)
        }
    });

    let mut budget = limit_bytes;
    let mut out = Vec::new();
    for (name, path) in files {
        if budget == 0 {
            break;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let slice = if text.len() > budget {
            let start = text
                .char_indices()
                .map(|(index, _)| index)
                .find(|index| *index >= text.len() - budget)
                .unwrap_or(0);
            &text[start..]
        } else {
            text.as_str()
        };
        budget = budget.saturating_sub(slice.len());
        out.push((name, slice.to_string()));
    }
    out
}

fn append(
    dir: &Path,
    level: Level,
    target: &str,
    message: &str,
    max_bytes: u64,
    max_rotated: usize,
) -> Option<()> {
    rotate_if_needed(dir, max_bytes, max_rotated);
    let path = dir.join(LOG_NAME);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    writeln!(
        file,
        "{} [{}] [{}] {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        level.label(),
        target,
        one_line(message)
    )
    .ok()?;
    Some(())
}

fn one_line(message: &str) -> String {
    message.replace(['\r', '\n'], " ").trim().to_string()
}

/// Rota si el archivo actual supera `max_bytes`: el más viejo se descarta y el
/// resto avanza un número antes de reciclar el actual como `.1`.
fn rotate_if_needed(dir: &Path, max_bytes: u64, max_rotated: usize) {
    let path = dir.join(LOG_NAME);
    let Ok(meta) = fs::metadata(&path) else {
        return;
    };
    if meta.len() < max_bytes {
        return;
    }
    for index in (1..max_rotated).rev() {
        let from = dir.join(format!("isalab.{index}.log"));
        let to = dir.join(format!("isalab.{}.log", index + 1));
        if from.exists() {
            let _ = fs::rename(&from, &to);
        }
    }
    let _ = fs::rename(&path, dir.join("isalab.1.log"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isalab-log-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("crear dir temporal");
        dir
    }

    #[test]
    fn escribe_una_linea_con_fecha_nivel_y_objetivo() {
        let dir = temp_dir("linea");
        append(
            &dir,
            Level::Error,
            "frontend",
            "falló algo",
            MAX_BYTES,
            MAX_ROTATED,
        );
        let text = fs::read_to_string(dir.join(LOG_NAME)).expect("leer log");
        assert!(
            text.contains("[ERROR] [frontend] falló algo"),
            "log: {text}"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn aplana_los_saltos_de_linea_de_un_mensaje() {
        let dir = temp_dir("saltos");
        append(
            &dir,
            Level::Warn,
            "db",
            "primera\nsegunda\r\ntercera",
            MAX_BYTES,
            MAX_ROTATED,
        );
        let text = fs::read_to_string(dir.join(LOG_NAME)).expect("leer log");
        assert_eq!(text.lines().count(), 1, "log: {text}");
        assert!(text.contains("primera segunda  tercera"), "log: {text}");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rota_el_archivo_actual_al_superar_el_tamano() {
        let dir = temp_dir("rotacion");
        append(&dir, Level::Info, "app", "primera", 40, MAX_ROTATED);
        // Con un límite de 40 bytes, la segunda escritura rota el actual.
        append(&dir, Level::Info, "app", "segunda", 40, MAX_ROTATED);

        let rotated = dir.join("isalab.1.log");
        assert!(rotated.exists(), "no se creó isalab.1.log");
        let old = fs::read_to_string(&rotated).expect("leer rotado");
        assert!(old.contains("primera"), "rotado: {old}");
        let current = fs::read_to_string(dir.join(LOG_NAME)).expect("leer actual");
        assert!(current.contains("segunda"), "actual: {current}");
        assert!(!current.contains("primera"), "actual: {current}");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn interpreta_el_nivel_que_manda_la_ui() {
        assert_eq!(Level::parse("error"), Level::Error);
        assert_eq!(Level::parse("WARNING"), Level::Warn);
        assert_eq!(Level::parse("info"), Level::Info);
        // Un valor inesperado no se pierde: queda como info.
        assert_eq!(Level::parse("debug"), Level::Info);
    }
}
