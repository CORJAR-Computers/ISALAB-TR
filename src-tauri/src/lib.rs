#![allow(linker_messages)]

pub mod ai_cache;
pub mod auth;
pub mod commands;
pub mod crypto;
pub mod csv;
pub mod csv_parse;
pub mod db;
pub mod error;
pub mod logging;
pub mod models;
pub mod pdf_templates;
pub mod repositories;
pub mod sources;
pub mod state;
#[cfg(test)]
pub mod test_helpers;
pub mod window_state;

// Solo se usa para regenerar src/bindings.ts en builds de desarrollo.
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri::{Manager, WindowEvent};
use tauri_specta::{collect_commands, Builder};

mod mail;

use crate::commands::ai::{interpret_lab_results, test_groq_connection};
use crate::commands::analyzer_sources::{
    delete_analyzer_import_job, delete_analyzer_source, list_analyzer_import_jobs,
    list_analyzer_sources, list_failed_analyzer_imports, poll_analyzer_source,
    save_analyzer_source,
};
use crate::commands::analyzers::{
    create_analyzer, create_reference_range, delete_analyzer, delete_reference_range,
    list_analyzers, list_reference_ranges, set_analyzer_active, update_analyzer,
    update_reference_range,
};
use crate::commands::appointments::{
    cancel_appointment, check_in_appointment, create_appointment, get_appointment,
    list_appointments, set_appointment_status,
};
use crate::commands::attachments::{
    attach_result_file, delete_result_attachment, get_result_attachment_content,
};
use crate::commands::auth::{get_session, list_audit_log, login, logout};
use crate::commands::cash::{
    add_cash_movement, close_cash_session, get_cash_session_summary, get_open_cash_session,
    list_cash_movements, list_cash_sessions, open_cash_session,
};
use crate::commands::catalog::{
    create_analyte, list_analytes, list_breeds, list_sample_types, list_species, list_vaccine_types,
};
use crate::commands::clinical_history::{
    count_consultations, create_consultation, get_clinical_history, list_consultations,
    set_consultation_status,
};
use crate::commands::dashboard::get_dashboard_stats;
use crate::commands::db::{
    db_health, export_local_config, get_db_connection, get_server_backup_status,
    save_db_connection, test_db_connection,
};
use crate::commands::diagnostics::{export_diagnostics, log_frontend};
use crate::commands::exports::{export_results_csv, export_samples_csv};
use crate::commands::grooming::{
    add_visit_attachment, delete_visit_attachment, finish_grooming_session, get_grooming_session,
    get_visit_attachment_content, list_grooming_day, list_grooming_reminders,
    list_visit_attachments, open_visit_attachment, upsert_grooming_session,
};
use crate::commands::import::{import_analyzer_results, preview_analyzer_import};
use crate::commands::invoices::{
    count_invoices, create_invoice, get_invoice, list_invoices, set_invoice_status,
};
use crate::commands::lab_orders::{
    accession_lab_order, count_lab_orders, create_lab_order, get_lab_order, get_order_for_sample,
    list_lab_orders, list_patient_lab_orders, set_lab_order_status,
};
use crate::commands::notifications::{
    acknowledge_critical, list_sample_notifications, send_critical_email, test_smtp_connection,
};
use crate::commands::panels::{delete_panel, list_panel_analytes, list_panels, save_panel};
use crate::commands::patients::{
    create_patient, get_patient, get_patient_by_code, get_patient_lab_trends, list_owners,
    list_patients,
};
use crate::commands::payments::{
    get_billing_report, get_invoice_balance, list_invoice_payments, list_payment_methods,
    register_payment, void_payment,
};
use crate::commands::products::{
    add_stock_movement, get_product, list_products, list_stock_movements, save_product,
    set_product_active,
};
use crate::commands::qc::{
    delete_qc_material, delete_qc_run, get_qc_chart, list_qc_analyzer_status, list_qc_materials,
    list_qc_runs, list_qc_targets, record_qc_run, save_qc_material,
};
use crate::commands::reports::{
    generate_carnet_vacunacion, generate_cash_close_pdf, generate_certificado_cirugia,
    generate_clinical_report, generate_consentimiento, generate_formula_medica,
    generate_recibo_invoice, generate_sample_labels, list_reports, open_report_file,
};
use crate::commands::samples::{
    count_samples, create_sample, delete_lab_result, get_sample, get_worklist, list_sample_events,
    list_samples, register_lab_result, register_lab_results, reject_sample, reopen_sample,
    set_sample_quality, set_sample_status,
};
use crate::commands::search::global_search;
use crate::commands::services::{
    delete_service_category, get_service, list_all_service_categories, list_service_categories,
    list_services, save_service, save_service_category, set_service_active,
};
use crate::commands::settings::{
    delete_clinic_logo, delete_pkcs12, delete_secondary_logo, get_clinic_branding,
    get_clinic_logo_content, get_clinic_settings, get_pkcs12_content, get_secondary_logo_content,
    import_clinic_logo, import_pkcs12, import_secondary_logo, list_secondary_logos,
    save_clinic_settings,
};
use crate::commands::stays::{
    add_stay_event, cancel_stay, check_out_stay, create_stay, get_stay, list_resource_board,
    list_resources, list_stay_events, list_stays, save_resource,
};
use crate::commands::surgeries::{
    count_surgeries, create_surgery, list_surgeries, set_surgery_status,
};
use crate::commands::users::{change_password, create_user, list_users};
use crate::commands::vaccines::{create_vaccine, list_vaccines};
use crate::commands::visits::{
    add_visit_service, checkout_visit, create_visit, get_visit, list_visits, remove_visit_service,
    set_visit_status, update_visit_service_status,
};
use crate::state::AppState;

/// Dimensiones (lógicas) de la ventana tras recortarlas al área de trabajo
/// del monitor donde se encuentra. Devuelve `None` si la ventana ya cabe:
/// en ese caso no se redimensiona y las pantallas grandes quedan intactas.
///
/// `min` son los mínimos del config ya recortados al área de trabajo (el
/// llamador aplica el piso absoluto 560x480).
fn fitted_size(cur: (f64, f64), wa: (f64, f64), min: (f64, f64)) -> Option<(f64, f64)> {
    // Tolerancia de medio px lógico: no redimensionar por redondeos de DPI.
    if cur.0 <= wa.0 + 0.5 && cur.1 <= wa.1 + 0.5 {
        return None;
    }
    Some((cur.0.min(wa.0).max(min.0), cur.1.min(wa.1).max(min.1)))
}

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            db_health,
            log_frontend,
            export_diagnostics,
            get_db_connection,
            save_db_connection,
            test_db_connection,
            global_search,
            export_local_config,
            get_server_backup_status,
            export_samples_csv,
            export_results_csv,
            list_service_categories,
            list_all_service_categories,
            save_service_category,
            delete_service_category,
            list_services,
            get_service,
            save_service,
            set_service_active,
            list_visits,
            get_visit,
            create_visit,
            set_visit_status,
            add_visit_service,
            remove_visit_service,
            update_visit_service_status,
            checkout_visit,
            list_appointments,
            get_appointment,
            create_appointment,
            set_appointment_status,
            cancel_appointment,
            check_in_appointment,
            get_grooming_session,
            upsert_grooming_session,
            finish_grooming_session,
            list_grooming_day,
            list_grooming_reminders,
            list_visit_attachments,
            add_visit_attachment,
            delete_visit_attachment,
            get_visit_attachment_content,
            open_visit_attachment,
            list_species,
            list_breeds,
            list_sample_types,
            list_analytes,
            create_analyte,
            list_vaccine_types,
            list_owners,
            list_patients,
            get_patient,
            get_patient_by_code,
            create_patient,
            get_clinical_history,
            create_consultation,
            create_sample,
            register_lab_result,
            register_lab_results,
            delete_lab_result,
            get_worklist,
            list_samples,
            count_samples,
            get_sample,
            set_sample_status,
            set_sample_quality,
            reject_sample,
            reopen_sample,
            list_sample_events,
            list_sample_notifications,
            acknowledge_critical,
            send_critical_email,
            test_smtp_connection,
            create_lab_order,
            list_lab_orders,
            list_patient_lab_orders,
            get_lab_order,
            count_lab_orders,
            set_lab_order_status,
            accession_lab_order,
            get_order_for_sample,
            preview_analyzer_import,
            import_analyzer_results,
            list_panels,
            list_panel_analytes,
            save_panel,
            delete_panel,
            list_qc_materials,
            list_qc_targets,
            save_qc_material,
            delete_qc_material,
            record_qc_run,
            list_qc_runs,
            delete_qc_run,
            get_qc_chart,
            list_qc_analyzer_status,
            get_clinic_branding,
            get_clinic_settings,
            save_clinic_settings,
            import_clinic_logo,
            import_pkcs12,
            list_secondary_logos,
            import_secondary_logo,
            delete_secondary_logo,
            get_clinic_logo_content,
            delete_clinic_logo,
            get_secondary_logo_content,
            import_pkcs12,
            delete_pkcs12,
            get_pkcs12_content,
            test_groq_connection,
            login,
            logout,
            get_session,
            generate_clinical_report,
            generate_formula_medica,
            generate_consentimiento,
            generate_recibo_invoice,
            generate_certificado_cirugia,
            generate_carnet_vacunacion,
            generate_sample_labels,
            list_reports,
            open_report_file,
            list_users,
            create_user,
            change_password,
            count_consultations,
            list_consultations,
            set_consultation_status,
            create_vaccine,
            list_vaccines,
            create_surgery,
            list_surgeries,
            count_surgeries,
            set_surgery_status,
            create_invoice,
            list_invoices,
            count_invoices,
            get_invoice,
            set_invoice_status,
            list_payment_methods,
            register_payment,
            void_payment,
            get_billing_report,
            get_invoice_balance,
            list_invoice_payments,
            get_open_cash_session,
            open_cash_session,
            add_cash_movement,
            list_cash_movements,
            get_cash_session_summary,
            close_cash_session,
            list_cash_sessions,
            generate_cash_close_pdf,
            get_dashboard_stats,
            attach_result_file,
            delete_result_attachment,
            get_result_attachment_content,
            list_audit_log,
            interpret_lab_results,
            get_patient_lab_trends,
            list_analyzers,
            create_analyzer,
            update_analyzer,
            set_analyzer_active,
            delete_analyzer,
            list_reference_ranges,
            create_reference_range,
            update_reference_range,
            delete_reference_range,
            list_analyzer_sources,
            save_analyzer_source,
            delete_analyzer_source,
            poll_analyzer_source,
            list_analyzer_import_jobs,
            list_failed_analyzer_imports,
            delete_analyzer_import_job,
            // Estancia e inventario (fase 6).
            list_resources,
            list_resource_board,
            save_resource,
            list_stays,
            get_stay,
            create_stay,
            check_out_stay,
            cancel_stay,
            add_stay_event,
            list_stay_events,
            list_products,
            get_product,
            save_product,
            set_product_active,
            add_stock_movement,
            list_stock_movements,
        ])
        // Tipos expuestos para la UI (eventos Firebird → Tauri, auditoría).
        .typ::<crate::models::sample::SampleChangedEvent>()
        .typ::<crate::models::sample::LabResultChangedEvent>()
        .typ::<crate::models::sample::SampleEvent>()
        .typ::<crate::models::notification::NotificationLogEntry>()
        .typ::<crate::models::lab_order::LabOrder>()
        .typ::<crate::models::lab_order::LabOrderItem>()
        .typ::<crate::models::lab_order::LabOrderListItem>()
        .typ::<crate::models::lab_order::OrderSampleRef>()
        .typ::<crate::models::lab_order::CreateLabOrderInput>()
        .typ::<crate::models::lab_order::CreateLabOrderItemInput>()
        .typ::<crate::models::lab_order::AccessionOrderInput>()
        .typ::<crate::models::auth::AuditLogEntry>()
        .typ::<crate::models::analyzer_source::AnalyzerSource>()
        .typ::<crate::models::analyzer_source::SaveAnalyzerSourceInput>()
        .typ::<crate::models::analyzer_source::AnalyzerImportJob>()
        // Catálogo de servicios (fase clínica 1).
        .typ::<crate::models::service::ServiceCategory>()
        .typ::<crate::models::service::Service>()
        .typ::<crate::models::service::SaveServiceCategoryInput>()
        .typ::<crate::models::service::SaveServiceInput>()
        // Recepción de la clínica (fase 2).
        .typ::<crate::models::visit::Visit>()
        .typ::<crate::models::visit::VisitService>()
        .typ::<crate::models::visit::VisitListItem>()
        .typ::<crate::models::visit::CreateVisitInput>()
        .typ::<crate::models::visit::AddVisitServiceInput>()
        .typ::<crate::models::visit::UpdateVisitServiceStatusInput>()
        // Agenda generalizada (fase 3).
        .typ::<crate::models::appointment::Appointment>()
        .typ::<crate::models::appointment::AppointmentListItem>()
        .typ::<crate::models::appointment::CreateAppointmentInput>()
        .typ::<crate::models::appointment::UpdateAppointmentStatusInput>()
        // Peluquería (fase 4).
        .typ::<crate::models::grooming::GroomingSession>()
        .typ::<crate::models::grooming::UpsertGroomingSessionInput>()
        .typ::<crate::models::grooming::FinishGroomingSessionInput>()
        .typ::<crate::models::grooming::VisitAttachment>()
        .typ::<crate::models::grooming::AddVisitAttachmentInput>()
        .typ::<crate::models::grooming::GroomingDayLine>()
        .typ::<crate::models::grooming::GroomingReminder>()
        // Archivos y certificados como BLOB (Fase 6).
        .typ::<crate::models::attachment::AttachmentContent>()
        .typ::<crate::models::settings::ClinicAsset>()
        .typ::<crate::commands::db::BackupStatus>()
        // Estancia e inventario (fase 6).
        .typ::<crate::models::stay::Resource>()
        .typ::<crate::models::stay::ResourceBoardItem>()
        .typ::<crate::models::stay::Stay>()
        .typ::<crate::models::stay::StayListItem>()
        .typ::<crate::models::stay::StayEvent>()
        .typ::<crate::models::stay::CreateStayInput>()
        .typ::<crate::models::stay::CheckOutStayInput>()
        .typ::<crate::models::stay::CreateStayEventInput>()
        .typ::<crate::models::stay::SaveResourceInput>()
        .typ::<crate::models::product::Product>()
        .typ::<crate::models::product::ProductStockItem>()
        .typ::<crate::models::product::StockMovement>()
        .typ::<crate::models::product::SaveProductInput>()
        .typ::<crate::models::product::CreateStockMovementInput>()
        // Cobros, abonos y caja (fase 7).
        .typ::<crate::models::payment::PaymentMethod>()
        .typ::<crate::models::payment::PaymentLine>()
        .typ::<crate::models::payment::PaymentAllocation>()
        .typ::<crate::models::payment::Payment>()
        .typ::<crate::models::payment::InvoiceBalance>()
        .typ::<crate::models::payment::CreatePaymentLineInput>()
        .typ::<crate::models::payment::CreatePaymentAllocationInput>()
        .typ::<crate::models::payment::RegisterPaymentInput>()
        .typ::<crate::models::payment::BillingMethodTotal>()
        .typ::<crate::models::payment::BillingReport>()
        .typ::<crate::models::cash::CashSession>()
        .typ::<crate::models::cash::CashMovement>()
        .typ::<crate::models::cash::CashMethodTotal>()
        .typ::<crate::models::cash::CashCountLine>()
        .typ::<crate::models::cash::CashSessionSummary>()
        .typ::<crate::models::cash::AddCashMovementInput>()
        .typ::<crate::models::cash::CashCountInput>()
        .typ::<crate::models::cash::CloseCashInput>()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Regenera src/bindings.ts (solo en dev; el repo conserva una copia).
    #[cfg(debug_assertions)]
    {
        specta_builder()
            .export(Typescript::default(), "../src/bindings.ts")
            .expect("Fallo al exportar bindings TypeScript");
    }

    tauri::Builder::default()
        // Persistencia: la geometría final de la ventana principal (tamaño,
        // posición y maximizado) se guarda justo antes de cerrarse, para
        // restaurarla en la próxima sesión.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    window_state::save_current(window.app_handle());
                }
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Log rotativo en <app_data>/logs antes de tocar la base: si el
            // arranque falla, el motivo queda en disco y no solo en la UI.
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            crate::logging::init(&app_data);
            crate::logging::info(
                "app",
                &format!("iniciando ISALAB {}", app.package_info().version),
            );

            let state = AppState::init(app.handle());
            // Supervisor de fuentes de analizadores (carpetas vigiladas) en
            // segundo plano; sondea las fuentes habilitadas cada 3 s.
            crate::sources::start_supervisor(state.pool.clone());
            app.manage(state);

            // Splash screen: la ventana principal arranca oculta; la UI avisa
            // con el evento "app-ready" cuando está lista y entonces se
            // muestra la principal y se cierra la ventana splash.
            #[cfg(desktop)]
            {
                use std::time::Duration;
                use tauri::{Listener, LogicalPosition, LogicalSize};

                let splash = app.get_webview_window("splash");
                let main = app.get_webview_window("main");

                // Portátiles pequeños (p. ej. 14" con 1366x768 a 125 % de
                // escala): el área de trabajo (~1093x576 px lógicos con barra
                // de tareas) es menor que el alto inicial (820) y que el
                // mínimo de 640 del config, y la ventana quedaría parcialmente
                // fuera de la pantalla e inaccesible. Se recorta tamaño y
                // posición al área de trabajo del monitor donde esté la
                // ventana. Si ya cabe, no se toca nada (pantallas grandes
                // quedan exactamente igual).
                //
                // Antes se intenta restaurar la geometría persistida de la
                // sesión anterior (window-state.json en app_data): tamaño y
                // posición en px físicos, recortados al área de trabajo del
                // monitor que contenga el centro guardado. Solo si no hay
                // estado (primer arranque) o no se pudo aplicar, se ejecuta el
                // recorte por defecto de arriba.
                let restored = main.as_ref().is_some_and(|win| {
                    window_state::state_path(app.handle())
                        .and_then(|path| window_state::load(&path))
                        .is_some_and(|state| window_state::apply(win, &state))
                });
                if let Some(win) = main.as_ref().filter(|_| !restored) {
                    let monitor = win
                        .current_monitor()
                        .ok()
                        .flatten()
                        .or_else(|| win.monitor_from_point(0.0, 0.0).ok().flatten());
                    if let Some(monitor) = monitor {
                        let scale = monitor.scale_factor();
                        let wa = monitor.work_area();
                        let (wa_x, wa_y) =
                            (wa.position.x as f64 / scale, wa.position.y as f64 / scale);
                        let (wa_w, wa_h) =
                            (wa.size.width as f64 / scale, wa.size.height as f64 / scale);
                        // Mínimos del config (1024x640) recortados al área de
                        // trabajo, con piso absoluto para no ser inusables.
                        let min_w = 1024.0_f64.min(wa_w).max(560.0);
                        let min_h = 640.0_f64.min(wa_h).max(480.0);
                        let cur = win
                            .inner_size()
                            .unwrap_or_default()
                            .to_logical::<f64>(scale);
                        if let Some((new_w, new_h)) =
                            fitted_size((cur.width, cur.height), (wa_w, wa_h), (min_w, min_h))
                        {
                            // Primero relajar el mínimo, luego el tamaño y por
                            // último recentrar dentro del área de trabajo.
                            let _ = win.set_min_size(Some(LogicalSize::new(min_w, min_h)));
                            let _ = win.set_size(LogicalSize::new(new_w, new_h));
                            let _ = win.set_position(LogicalPosition::new(
                                wa_x + (wa_w - new_w) / 2.0,
                                wa_y + (wa_h - new_h) / 2.0,
                            ));
                        }
                    }
                }

                if let (Some(splash), Some(main)) = (splash, main) {
                    let main_handle = main.clone();
                    let splash_handle = splash.clone();
                    let app_handle = app.handle().clone();
                    let fallback_app = app_handle.clone();

                    app.listen_any("app-ready", move |_| {
                        let _ = main_handle.show();
                        let _ = main_handle.set_focus();
                        let _ = splash_handle.close();
                        // Con la ventana ya visible se puede restaurar el
                        // maximizado guardado (ver window_state.rs).
                        window_state::apply_maximized_if_needed(&app_handle);
                    });

                    // Red de seguridad: si la UI nunca emite "app-ready",
                    // muestra la ventana principal igualmente.
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(10));
                        let _ = main.show();
                        let _ = splash.close();
                        window_state::apply_maximized_if_needed(&fallback_app);
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(specta_builder().invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod window_fit_tests {
    use super::fitted_size;

    #[test]
    fn keeps_size_when_the_window_already_fits() {
        assert_eq!(
            fitted_size((1280.0, 820.0), (1600.0, 900.0), (1024.0, 640.0)),
            None
        );
    }

    #[test]
    fn shrinks_to_the_work_area_on_a_small_laptop() {
        // 1366x768 físicos a 125 %: ~1092.8x576 lógicos de área de trabajo.
        assert_eq!(
            fitted_size((1280.0, 820.0), (1092.8, 576.0), (1024.0, 480.0)),
            Some((1092.8, 576.0))
        );
    }

    #[test]
    fn respects_the_absolute_floor_on_tiny_monitors() {
        assert_eq!(
            fitted_size((1280.0, 820.0), (500.0, 400.0), (560.0, 480.0)),
            Some((560.0, 480.0))
        );
    }

    #[test]
    fn shrinks_only_the_offending_axis() {
        assert_eq!(
            fitted_size((900.0, 820.0), (1024.0, 600.0), (560.0, 480.0)),
            Some((900.0, 600.0))
        );
    }
}
