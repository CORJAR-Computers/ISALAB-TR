//! Persistencia del tamaño/posición de la ventana principal entre sesiones.
//!
//! Se guarda un JSON en `app_data/window-state.json` con coordenadas EN PÍXELES
//! FÍSICOS (el espacio global de coordenadas del SO en Windows): evita la
//! ambigüedad de las coordenadas lógicas en configuraciones multi-monitor con
//! escalados distintos. Al restaurar, el rectángulo guardado se recorta al área
//! de trabajo del monitor donde cae su centro: idempotente en monitores grandes
//! y seguro si el monitor cambió o se desconectó desde la sesión anterior.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const FILE_NAME: &str = "window-state.json";

/// Geometría de la ventana principal (px físicos) + estado maximizado.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub maximized: bool,
}

/// Carga el estado guardado; `None` si no existe o el JSON está corrupto.
pub fn load(path: &Path) -> Option<WindowState> {
    let raw = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&raw) {
        Ok(state) => Some(state),
        Err(err) => {
            eprintln!("[window-state] JSON corrupto ({}): {err}", path.display());
            None
        }
    }
}

/// Guarda el estado (best-effort: un fallo de disco no debe romper la salida).
pub fn save(path: &Path, state: &WindowState) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let result = serde_json::to_string_pretty(state)
        .map_err(|e| e.to_string())
        .and_then(|json| std::fs::write(path, json).map_err(|e| e.to_string()));
    if let Err(err) = result {
        eprintln!(
            "[window-state] no se pudo guardar ({}): {err}",
            path.display()
        );
    }
}

/// Posición (px físicos) ajustada para que la ventana quede mayormente visible
/// dentro del área de trabajo: al menos `MIN_VISIBLE` px de cada eje dentro del
/// área (o todo el eje si la ventana es más grande que el área de trabajo).
pub fn clamped_position(
    pos: (f64, f64),
    size: (f64, f64),
    wa_pos: (f64, f64),
    wa_size: (f64, f64),
) -> (f64, f64) {
    const MIN_VISIBLE: f64 = 120.0;
    let axis = |p: f64, s: f64, wa_p: f64, wa_s: f64| -> f64 {
        if s >= wa_s {
            // Más grande que el área: pegada a la esquina del área.
            return wa_p;
        }
        let min_visible = MIN_VISIBLE.min(s);
        let lower = wa_p - (s - min_visible);
        let upper = wa_p + wa_s - min_visible;
        p.clamp(lower, upper)
    };
    (
        axis(pos.0, size.0, wa_pos.0, wa_size.0),
        axis(pos.1, size.1, wa_pos.1, wa_size.1),
    )
}

/// Restaura `state` sobre `win`, recortado al monitor que contiene el centro
/// guardado. Devuelve `false` si no se pudo determinar el monitor (el llamador
/// aplicará entonces su comportamiento por defecto). El flag `maximized` NO se
/// aplica aquí: el maximizado de Windows fuerza la visibilidad (ShowWindow),
/// lo que rompería el arranque oculto tras el splash; el llamador lo aplica
/// cuando la ventana ya es visible (ver `apply_maximized_if_needed`).
pub fn apply(win: &WebviewWindow, state: &WindowState) -> bool {
    let center_x = state.x as f64 + state.width / 2.0;
    let center_y = state.y as f64 + state.height / 2.0;
    let Ok(Some(monitor)) = win.monitor_from_point(center_x, center_y) else {
        return false;
    };
    let scale = monitor.scale_factor();
    let wa = monitor.work_area();
    let (wa_x, wa_y) = (wa.position.x as f64, wa.position.y as f64);
    let (wa_w, wa_h) = (wa.size.width as f64, wa.size.height as f64);

    // Mínimos del config (560x480 lógicos) en px físicos de ESTE monitor.
    let min_w = 560.0 * scale;
    let min_h = 480.0 * scale;
    // Tamaño guardado recortado al área de trabajo (sin bajar de los mínimos;
    // `wa.max(min)` evita el panic de `clamp` si el área es menor que el mínimo,
    // en cuyo caso la ventana sobresale lo mismo que con el mín. del config).
    let w = state.width.clamp(min_w, wa_w.max(min_w));
    let h = state.height.clamp(min_h, wa_h.max(min_h));
    let (x, y) = clamped_position(
        (state.x as f64, state.y as f64),
        (w, h),
        (wa_x, wa_y),
        (wa_w, wa_h),
    );

    let _ = win.set_size(PhysicalSize::new(w.round() as u32, h.round() as u32));
    let _ = win.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
    true
}

/// Aplica el flag `maximized` del estado guardado si la ventana principal ya
/// es visible. Debe llamarse tras el evento "app-ready": con la ventana aún
/// oculta, maximizar forzaría su visibilidad (ShowWindow) y rompería el
/// splash. Sin estado guardado no hace nada.
pub fn apply_maximized_if_needed(app: &tauri::AppHandle) {
    let Some(path) = state_path(app) else {
        return;
    };
    let Some(state) = load(&path) else {
        return;
    };
    if !state.maximized {
        return;
    }
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.maximize();
        }
    }
}

/// Captura la geometría actual de la ventana (px físicos). `None` si está
/// minimizada (Windows reporta las coordenadas icónicas -32000, basura para
/// restaurar) o si falla la lectura.
pub fn capture(win: &WebviewWindow) -> Option<WindowState> {
    if win.is_minimized().unwrap_or(true) {
        return None;
    }
    let size = win.inner_size().ok()?;
    let pos = win.outer_position().ok()?;
    Some(WindowState {
        width: size.width as f64,
        height: size.height as f64,
        x: pos.x,
        y: pos.y,
        maximized: win.is_maximized().unwrap_or(false),
    })
}

/// Ruta del archivo de estado (`app_data/window-state.json`).
pub fn state_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join(FILE_NAME))
}

/// Guarda la geometría actual de la ventana principal (llamar al salir).
pub fn save_current(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let Some(path) = state_path(app) else {
        return;
    };
    if let Some(state) = capture(&win) {
        save(&path, &state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let state = WindowState {
            width: 1280.0,
            height: 820.0,
            x: 42,
            y: -7,
            maximized: true,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(serde_json::from_str::<WindowState>(&json).unwrap(), state);
    }

    #[test]
    fn maximized_defaults_to_false() {
        let state: WindowState =
            serde_json::from_str(r#"{"width":100.0,"height":100.0,"x":0,"y":0}"#).unwrap();
        assert!(!state.maximized);
    }

    #[test]
    fn position_unchanged_when_inside_work_area() {
        assert_eq!(
            clamped_position((100.0, 100.0), (800.0, 600.0), (0.0, 0.0), (1920.0, 1040.0)),
            (100.0, 100.0)
        );
    }

    #[test]
    fn position_pulled_back_when_off_screen_right() {
        let (x, y) = clamped_position(
            (2500.0, 100.0),
            (800.0, 600.0),
            (0.0, 0.0),
            (1920.0, 1040.0),
        );
        assert_eq!(x, 1920.0 - 120.0);
        assert_eq!(y, 100.0);
    }

    #[test]
    fn position_keeps_min_visible_when_off_screen_left() {
        let (x, _) = clamped_position((-5000.0, 0.0), (800.0, 600.0), (0.0, 0.0), (1920.0, 1040.0));
        assert_eq!(x, -(800.0 - 120.0));
    }

    #[test]
    fn oversized_window_pinned_to_work_area_origin() {
        assert_eq!(
            clamped_position(
                (-100.0, -100.0),
                (3000.0, 2000.0),
                (0.0, 0.0),
                (1920.0, 1040.0)
            ),
            (0.0, 0.0)
        );
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "isalab-window-state-test-{}.json",
            std::process::id()
        ));
        let state = WindowState {
            width: 1280.0,
            height: 820.0,
            x: 42,
            y: -7,
            maximized: true,
        };
        save(&path, &state);
        assert_eq!(load(&path), Some(state));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_missing_or_corrupt_returns_none() {
        assert_eq!(load(Path::new("Z:/definitivamente/no/existe.json")), None);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "isalab-window-state-corrupt-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, "{ no es json").unwrap();
        assert_eq!(load(&path), None);
        let _ = std::fs::remove_file(&path);
    }
}
