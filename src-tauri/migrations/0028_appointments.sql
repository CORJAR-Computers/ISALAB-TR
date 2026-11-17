-- ============================================================================
-- ISALAB · Migración 0028 — Agenda generalizada (APPOINTMENTS)
--
-- Citas de cualquier servicio del catálogo (no solo consultas), con duración,
-- profesional asignado, franja horaria y máquina de estados. Una cita puede
-- convertirse en visita (check-in): se crea la VISITA con APPOINTMENT_ID y
-- la cita pasa a ATENDIDA.
--
-- Validación de solapamiento (mismo profesional, intervalos que se cruzan,
-- estados no cancelados) se hace en Rust: Firebird no puede expresarla con
-- un CHECK.
-- ============================================================================

CREATE TABLE APPOINTMENTS (
    ID              D_PK NOT NULL PRIMARY KEY,
    -- Código C-YYYY-NNNN (trigger BI_APPOINTMENTS).
    CODE            D_CODE NOT NULL UNIQUE,
    PATIENT_ID      D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    -- Propietario: se resuelve a través del paciente (no se duplica).
    SERVICE_ID      D_PK NOT NULL REFERENCES SERVICES (ID),
    PROFESSIONAL_ID D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    SCHEDULED_AT    TIMESTAMP NOT NULL,
    DURATION_MIN    INTEGER NOT NULL CHECK (DURATION_MIN > 0),
    -- PROGRAMADA | CONFIRMADA | EN_ATENCION | ATENDIDA | NO_ASISTIO | CANCELADA
    STATUS          VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'PROGRAMADA' NOT NULL,
    NOTES           D_NOTES,
    CREATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_APPOINTMENTS_ID;

CREATE INDEX IX_APPOINTMENTS_SCHEDULED_PROF ON APPOINTMENTS (SCHEDULED_AT, PROFESSIONAL_ID);

-- Vínculo visita ← cita (la columna APPOINTMENT_ID ya existía en VISITS;
-- aquí se añade la FK).
ALTER TABLE VISITS
    ADD CONSTRAINT FK_VISITS_APPOINTMENT
    FOREIGN KEY (APPOINTMENT_ID) REFERENCES APPOINTMENTS (ID) ON DELETE SET NULL;

SET TERM ^ ;

CREATE TRIGGER BI_APPOINTMENTS FOR APPOINTMENTS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_APPOINTMENTS_ID, 1);
    IF (NEW.CODE IS NULL) THEN
        NEW.CODE = 'C-' || EXTRACT(YEAR FROM NEW.SCHEDULED_AT) || '-' ||
                   LPAD(NEW.ID, 4, '0');
END^

SET TERM ; ^
