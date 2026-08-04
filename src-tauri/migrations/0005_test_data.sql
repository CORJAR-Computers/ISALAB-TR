-- ============================================================================
-- ISALAB · Migración 0005 — Datos de prueba (Pacientes, Muestras, Consultas,
-- Cirugías y Facturas para demostración y evaluación del sistema).
-- ============================================================================

-- Propietarios de prueba
INSERT INTO OWNERS (ID, FULL_NAME, DOCUMENT_TYPE, DOCUMENT_NUMBER, PHONE, EMAIL, ADDRESS, CITY)
VALUES (1, 'Carlos Andrés Mendoza', 'CC', '1020304050', '3105551234', 'carlos.mendoza@gmail.com', 'Calle 10 # 43-20', 'Medellín');

INSERT INTO OWNERS (ID, FULL_NAME, DOCUMENT_TYPE, DOCUMENT_NUMBER, PHONE, EMAIL, ADDRESS, CITY)
VALUES (2, 'María Fernanda Gómez', 'CC', '52431980', '3124449876', 'mfgomez@hotmail.com', 'Carrera 25 # 12-50', 'Envigado');

INSERT INTO OWNERS (ID, FULL_NAME, DOCUMENT_TYPE, DOCUMENT_NUMBER, PHONE, EMAIL, ADDRESS, CITY)
VALUES (3, 'Criadero Ruffos House', 'NIT', '900123456-1', '3008887766', 'contacto@ruffoshouse.com', 'Vereda Las Palmas Km 5', 'Rionegro');

-- Pacientes de prueba
INSERT INTO PATIENTS (ID, OWNER_ID, SPECIES_ID, BREED_ID, NAME, SEX, BIRTH_DATE, NEUTERED, COLOR, MICROCHIP, NOTES)
VALUES (1, 1, 1, 2, 'Thor', 'M', '2024-05-10', TRUE, 'Dorado', '985141002345678', 'Paciente dócil, al día con esquema de vacunación.');

INSERT INTO PATIENTS (ID, OWNER_ID, SPECIES_ID, BREED_ID, NAME, SEX, BIRTH_DATE, NEUTERED, COLOR, MICROCHIP, NOTES)
VALUES (2, 2, 2, 11, 'Luna', 'F', '2024-11-15', TRUE, 'Crema / Punto Foca', '985141009876543', 'Revisión periódica de perfil renal.');

INSERT INTO PATIENTS (ID, OWNER_ID, SPECIES_ID, BREED_ID, NAME, SEX, BIRTH_DATE, NEUTERED, COLOR, MICROCHIP, NOTES)
VALUES (3, 3, 3, 19, 'Pegaso', 'M', '2022-03-20', FALSE, 'Castaño', '985141005554433', 'Ejemplar de paso fino colombiano.');

-- Consultas clínicas de prueba
INSERT INTO CONSULTATIONS (ID, PATIENT_ID, CONSULTATION_DATE, REASON, ANAMNESIS, PHYSICAL_EXAM, DIAGNOSIS, TREATMENT_PLAN, STATUS)
VALUES (1, 1, '2026-08-01 09:30:00', 'Chequeo preventivo anual y desparasitación', 'Paciente activo sin síntomas de dolor o inapetencia.', 'FC 85 bpm, FR 22 rpm, T 38.4 C, mucosas rosadas.', 'Paciente sano en estado óptimo.', 'Administración de antiparasitario oral y vacuna polivalente.', 'COMPLETADA');

INSERT INTO CONSULTATIONS (ID, PATIENT_ID, CONSULTATION_DATE, REASON, ANAMNESIS, PHYSICAL_EXAM, DIAGNOSIS, TREATMENT_PLAN, STATUS)
VALUES (2, 2, '2026-08-02 14:00:00', 'Inapetencia leve y revisión de piel', 'Presenta rascado en la zona lumbar desde hace 3 días.', 'FC 110 bpm, FR 28 rpm, T 38.6 C, alopecia focalizada.', 'Dermatitis alérgica por picadura de pulga (DAPP).', 'Tratamiento con pipeta antipulgas y champú antiséptico.', 'COMPLETADA');

-- Muestras analíticas y resultados
INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS, COLLECTED_BY, NOTES)
VALUES (1, 'M-2026-0001', 1, 1, '2026-08-01 10:00:00', 'FINALIZADA', 'Dr. Alejandro Ruiz', 'Muestra tomada en tubo EDTA para hemograma completo.');

INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS, COLLECTED_BY, NOTES)
VALUES (2, 'M-2026-0002', 2, 2, '2026-08-02 14:30:00', 'FINALIZADA', 'Dra. Elena Valencia', 'Muestra de suero para perfil hepático y renal.');

-- Resultados analíticos
INSERT INTO LAB_RESULTS (ID, SAMPLE_ID, ANALYTE_ID, REFERENCE_RANGE_ID, RESULT_VALUE, STATUS, NOTES)
VALUES (1, 1, 1, 1, 15.2, 'NORMAL', 'Hemoglobina dentro de límites normales.');

INSERT INTO LAB_RESULTS (ID, SAMPLE_ID, ANALYTE_ID, REFERENCE_RANGE_ID, RESULT_VALUE, STATUS, NOTES)
VALUES (2, 1, 2, 2, 11.5, 'NORMAL', 'Conteo leucocitario adecuado.');

INSERT INTO LAB_RESULTS (ID, SAMPLE_ID, ANALYTE_ID, REFERENCE_RANGE_ID, RESULT_VALUE, STATUS, NOTES)
VALUES (3, 2, 3, 3, 1.2, 'NORMAL', 'Creatinina sérica en rango normal.');

-- Cirugías programadas
INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, ANESTHESIA_TYPE, PREOPERATIVE_NOTES, STATUS)
VALUES (1, 1, 'Profilaxis Dental y Tartrectomía', '2026-08-10 08:00:00', 'Anestesia General Inhalatoria (Isoflurano)', 'Ayuno de 8 horas. Evaluación cardíaca previa normal.', 'PROGRAMADA');

-- Facturas de prueba
INSERT INTO INVOICES (ID, INVOICE_NUMBER, PATIENT_ID, OWNER_ID, CONSULTATION_ID, ISSUE_DATE, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS, PAYMENT_METHOD, NOTES)
VALUES (1, 'F-2026-0001', 1, 1, 1, '2026-08-01 11:00:00', 150000.00, 19.00, 28500.00, 178500.00, 'PAGADA', 'TRANSFERENCIA', 'Pago confirmado por Bancolombia.');

INSERT INTO INVOICE_ITEMS (ID, INVOICE_ID, DESCRIPTION, QUANTITY, UNIT_PRICE, LINE_TOTAL)
VALUES (1, 1, 'Consulta médica veterinaria especializada', 1, 90000.00, 90000.00);

INSERT INTO INVOICE_ITEMS (ID, INVOICE_ID, DESCRIPTION, QUANTITY, UNIT_PRICE, LINE_TOTAL)
VALUES (2, 1, 'Facturación de laboratorio', 1, 60000.00, 60000.00);

-- Ajustar generadores de IDs en Firebird (sintaxis DDL estándar)
SET GENERATOR GEN_OWNERS_ID TO 10;
SET GENERATOR GEN_PATIENTS_ID TO 10;
SET GENERATOR GEN_CONSULTATIONS_ID TO 10;
SET GENERATOR GEN_SAMPLES_ID TO 10;
SET GENERATOR GEN_LAB_RESULTS_ID TO 10;
SET GENERATOR GEN_SURGERIES_ID TO 10;
SET GENERATOR GEN_INVOICES_ID TO 10;
SET GENERATOR GEN_INVOICE_ITEMS_ID TO 10;
