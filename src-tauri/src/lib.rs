#![allow(linker_messages)]

pub mod ai_cache;
pub mod auth;
pub mod commands;
pub mod crypto;
pub mod csv;
pub mod csv_parse;
pub mod db;
pub mod error;
pub mod models;
pub mod pdf_templates;
pub mod repositories;
pub mod sources;
pub mod state;

#[cfg(test)]
pub mod test_helpers;

// Solo se usa para regenerar src/bindings.ts en builds de desarrollo.
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use tauri::Manager;
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
use crate::commands::attachments::{attach_result_file, delete_result_attachment};
use crate::commands::auth::{get_session, list_audit_log, login, logout};
use crate::commands::catalog::{
    list_analytes, list_breeds, list_sample_types, list_species, list_vaccine_types,
};
use crate::commands::clinical_history::{
    count_consultations, create_consultation, get_clinical_history, list_consultations,
    set_consultation_status,
};
use crate::commands::dashboard::get_dashboard_stats;
use crate::commands::db::{create_local_backup, db_health};
use crate::commands::exports::{export_results_csv, export_samples_csv};
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
use crate::commands::qc::{
    delete_qc_material, delete_qc_run, get_qc_chart, list_qc_analyzer_status, list_qc_materials,
    list_qc_runs, list_qc_targets, record_qc_run, save_qc_material,
};
use crate::commands::reports::{
    generate_carnet_vacunacion, generate_certificado_cirugia, generate_clinical_report,
    generate_consentimiento, generate_formula_medica, generate_recibo_invoice,
    generate_sample_labels, list_reports, open_report_file,
};
use crate::commands::samples::{
    count_samples, create_sample, get_sample, get_worklist, list_sample_events, list_samples,
    register_lab_result, register_lab_results, reject_sample, reopen_sample, set_sample_quality,
    set_sample_status,
};
use crate::commands::search::global_search;
use crate::commands::settings::{
    delete_secondary_logo, get_clinic_settings, import_clinic_logo, import_pkcs12,
    import_secondary_logo, list_secondary_logos, save_clinic_settings,
};
use crate::commands::surgeries::{
    count_surgeries, create_surgery, list_surgeries, set_surgery_status,
};
use crate::commands::users::{change_password, create_user, list_users};
use crate::commands::vaccines::{create_vaccine, list_vaccines};
use crate::state::AppState;

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            db_health,
            global_search,
            create_local_backup,
            export_samples_csv,
            export_results_csv,
            list_species,
            list_breeds,
            list_sample_types,
            list_analytes,
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
            get_clinic_settings,
            save_clinic_settings,
            import_clinic_logo,
            import_pkcs12,
            list_secondary_logos,
            import_secondary_logo,
            delete_secondary_logo,
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
            get_dashboard_stats,
            attach_result_file,
            delete_result_attachment,
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
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
                use tauri::Listener;

                let splash = app.get_webview_window("splash");
                let main = app.get_webview_window("main");

                if let (Some(splash), Some(main)) = (splash, main) {
                    let main_handle = main.clone();
                    let splash_handle = splash.clone();

                    app.listen_any("app-ready", move |_| {
                        let _ = main_handle.show();
                        let _ = main_handle.set_focus();
                        let _ = splash_handle.close();
                    });

                    // Red de seguridad: si la UI nunca emite "app-ready",
                    // muestra la ventana principal igualmente.
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(10));
                        let _ = main.show();
                        let _ = splash.close();
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(specta_builder().invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
