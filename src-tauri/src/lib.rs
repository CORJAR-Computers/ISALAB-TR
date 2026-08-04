#![allow(linker_messages)]

pub mod auth;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod pdf_templates;
pub mod repositories;
pub mod state;

use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder};

use crate::commands::auth::{get_session, list_audit_log, login, logout};
use crate::commands::catalog::{
    list_analytes, list_breeds, list_sample_types, list_species, list_vaccine_types,
};
use crate::commands::clinical_history::{
    create_consultation, get_clinical_history, list_consultations, set_consultation_status,
};
use crate::commands::dashboard::get_dashboard_stats;
use crate::commands::db::db_health;
use crate::commands::invoices::{
    create_invoice, get_invoice, list_invoices, set_invoice_status,
};
use crate::commands::patients::{create_patient, get_patient, list_owners, list_patients};
use crate::commands::reports::{
    generate_carnet_vacunacion, generate_certificado_cirugia, generate_clinical_report,
    generate_consentimiento, generate_formula_medica, generate_recibo_invoice, list_reports,
    open_report_file,
};
use crate::commands::samples::{
    create_sample, get_sample, list_samples, register_lab_result, set_sample_status,
};
use crate::commands::settings::{
    get_clinic_settings, import_clinic_logo, save_clinic_settings,
};
use crate::commands::surgeries::{create_surgery, list_surgeries, set_surgery_status};
use crate::commands::users::{change_password, create_user, list_users};
use crate::commands::vaccines::{create_vaccine, list_vaccines};
use crate::commands::ai::interpret_lab_results;
use crate::state::AppState;

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            db_health,
            list_species,
            list_breeds,
            list_sample_types,
            list_analytes,
            list_vaccine_types,
            list_owners,
            list_patients,
            get_patient,
            create_patient,
            get_clinical_history,
            create_consultation,
            create_sample,
            register_lab_result,
            list_samples,
            get_sample,
            set_sample_status,
            get_clinic_settings,
            save_clinic_settings,
            import_clinic_logo,
            login,
            logout,
            get_session,
            generate_clinical_report,
            generate_formula_medica,
            generate_consentimiento,
            generate_recibo_invoice,
            generate_certificado_cirugia,
            generate_carnet_vacunacion,
            list_reports,
            open_report_file,
            list_users,
            create_user,
            change_password,
            list_consultations,
            set_consultation_status,
            create_vaccine,
            list_vaccines,
            create_surgery,
            list_surgeries,
            set_surgery_status,
            create_invoice,
            list_invoices,
            get_invoice,
            set_invoice_status,
            get_dashboard_stats,
            list_audit_log,
            interpret_lab_results,
        ])
        // Tipos expuestos para la UI (eventos Firebird → Tauri, auditoría).
        .typ::<crate::models::sample::SampleChangedEvent>()
        .typ::<crate::models::sample::LabResultChangedEvent>()
        .typ::<crate::models::auth::AuditLogEntry>()
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
        .setup(|app| {
            let state = AppState::init(app.handle());
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
