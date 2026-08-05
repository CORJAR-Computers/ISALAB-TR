//! Cache en memoria para interpretaciones de IA.
//!
//! Almacena las interpretaciones de Groq para evitar llamadas repetidas
//! a la API cuando los resultados no han cambiado.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// TTL por defecto para las interpretaciones cacheadas (24 horas).
const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Entrada del cache con timestamp y hash de resultados.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Texto de la interpretación.
    interpretation: String,
    /// Momento en que se guardó la entrada.
    created_at: Instant,
    /// Hash de los resultados para detectar cambios.
    results_hash: u64,
}

/// Cache thread-safe para interpretaciones de IA.
pub struct AiCache {
    entries: Mutex<HashMap<i32, CacheEntry>>,
    ttl: Duration,
}

impl AiCache {
    /// Crea un nuevo cache con TTL por defecto (24 horas).
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl: DEFAULT_TTL,
        }
    }

    /// Crea un nuevo cache con TTL personalizado.
    #[cfg(test)]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Obtiene una interpretación cacheada si existe y no ha expirado.
    /// Devuelve `None` si no está en cache o si el hash de resultados cambió.
    pub fn get(&self, sample_id: i32, results_hash: u64) -> Option<String> {
        let entries = self.entries.lock().ok()?;
        let entry = entries.get(&sample_id)?;

        // Verificar que no haya expirado
        if entry.created_at.elapsed() > self.ttl {
            return None;
        }

        // Verificar que el hash de resultados no haya cambiado
        if entry.results_hash != results_hash {
            return None;
        }

        Some(entry.interpretation.clone())
    }

    /// Almacena una interpretación en el cache.
    pub fn set(&self, sample_id: i32, interpretation: String, results_hash: u64) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(
                sample_id,
                CacheEntry {
                    interpretation,
                    created_at: Instant::now(),
                    results_hash,
                },
            );
        }
    }

    /// Invalida la entrada de un sample específico.
    pub fn invalidate(&self, sample_id: i32) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&sample_id);
        }
    }

    /// Limpia todas las entradas expiradas.
    pub fn cleanup(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.retain(|_, entry| entry.created_at.elapsed() <= self.ttl);
        }
    }

    /// Devuelve el número de entradas en el cache.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    /// Verifica si el cache está vacío.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for AiCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Calcula un hash simple de los resultados para detectar cambios.
pub fn hash_results(results: &[(f64, String)]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for (value, status) in results {
        value.to_bits().hash(&mut hasher);
        status.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_and_get() {
        let cache = AiCache::new();
        cache.set(1, "Interpretación de prueba".to_string(), 12345);

        let result = cache.get(1, 12345);
        assert_eq!(result, Some("Interpretación de prueba".to_string()));
    }

    #[test]
    fn test_cache_miss_on_different_hash() {
        let cache = AiCache::new();
        cache.set(1, "Interpretación".to_string(), 12345);

        let result = cache.get(1, 99999);
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_miss_on_expired_entry() {
        let cache = AiCache::with_ttl(Duration::from_millis(10));
        cache.set(1, "Interpretación".to_string(), 12345);

        // Esperar a que expire
        std::thread::sleep(Duration::from_millis(20));

        let result = cache.get(1, 12345);
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = AiCache::new();
        cache.set(1, "Interpretación".to_string(), 12345);
        assert!(cache.get(1, 12345).is_some());

        cache.invalidate(1);
        assert!(cache.get(1, 12345).is_none());
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = AiCache::with_ttl(Duration::from_millis(10));
        cache.set(1, "Uno".to_string(), 1);
        cache.set(2, "Dos".to_string(), 2);

        assert_eq!(cache.len(), 2);

        // Esperar a que expiren
        std::thread::sleep(Duration::from_millis(20));

        cache.cleanup();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_hash_results_deterministic() {
        let results = vec![(45.0, "NORMAL".to_string()), (120.0, "ALTO".to_string())];
        let hash1 = hash_results(&results);
        let hash2 = hash_results(&results);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_results_different_for_different_values() {
        let results1 = vec![(45.0, "NORMAL".to_string())];
        let results2 = vec![(50.0, "NORMAL".to_string())];
        assert_ne!(hash_results(&results1), hash_results(&results2));
    }

    #[test]
    fn test_hash_results_different_for_different_status() {
        let results1 = vec![(45.0, "NORMAL".to_string())];
        let results2 = vec![(45.0, "ALTO".to_string())];
        assert_ne!(hash_results(&results1), hash_results(&results2));
    }
}
