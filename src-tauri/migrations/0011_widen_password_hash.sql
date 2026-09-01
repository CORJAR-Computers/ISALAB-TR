-- ============================================================================
-- ISALAB · Migración 0011 — Ampliar PASSWORD_HASH a VARCHAR(255).
-- ----------------------------------------------------------------------------
-- La columna USERS.PASSWORD_HASH se definió como VARCHAR(100) en 0001, un
-- margen estrecho para un hash Argon2id codificado en formato PHC
-- (`$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`). Con el salt por defecto
-- (16 bytes) la cadena cabe (~97 chars), pero un cambio de parámetros de
-- Argon2 o un salt más largo haría que Firebird truncara silenciosamente el
-- hash, rompiendo la verificación y dejando al usuario (incluido `admin`)
-- sin posibilidad de iniciar sesión. VARCHAR(255) deja holgura suficiente.
-- ============================================================================

ALTER TABLE USERS ALTER COLUMN PASSWORD_HASH TYPE VARCHAR(255);
