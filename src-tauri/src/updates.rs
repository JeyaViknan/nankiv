//! Check for Updates.
//!
//! The one request nankiv makes to the network, and only when the student
//! asks: it fetches a small file from this project's GitHub Releases saying
//! what the newest version is. Nothing about the student goes with it, and
//! nothing is ever checked in the background — reminders come from how old
//! the installed build is, which needs no network at all.
//!
//! An update is downloaded by the app itself, its signature checked against
//! the public key compiled into this build, and installed over the copy that
//! is running: one nankiv afterwards, never two, and the data folder is left
//! exactly as it was. A download made by nankiv carries no browser quarantine,
//! so a Mac does not ask again whether the app can be opened.
//!
//! Releases are signed only once the signing key is configured in CI; until
//! then a check finds nothing to install, and the interface offers the
//! releases page — opened in the browser, so the app itself makes no request.

use crate::commands::CommandError;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize)]
pub struct AppVersion {
    pub version: String,
    /// When this copy of nankiv was built, RFC 3339 — read from the program
    /// file itself, so it is right however the app was built or installed.
    pub built: Option<String>,
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> AppVersion {
    let built = std::env::current_exe()
        .and_then(std::fs::metadata)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| {
            time::OffsetDateTime::from(t)
                .format(&time::format_description::well_known::Rfc3339)
                .ok()
        });
    AppVersion {
        version: app.package_info().version.to_string(),
        built,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateCheck {
    UpToDate,
    Available {
        version: String,
        notes: Option<String>,
    },
}

fn unreachable_error() -> CommandError {
    CommandError::new(
        "update_check_failed",
        "Couldn't check for updates. You can see every version on the releases page.",
    )
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateCheck, CommandError> {
    let updater = app.updater().map_err(|_| unreachable_error())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateCheck::Available {
            version: update.version.clone(),
            notes: update.body.clone(),
        }),
        Ok(None) => Ok(UpdateCheck::UpToDate),
        Err(_) => Err(unreachable_error()),
    }
}

/// Downloads the newest version, checks its signature, installs it over this
/// one and restarts into it.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), CommandError> {
    let updater = app.updater().map_err(|_| unreachable_error())?;
    let Some(update) = updater.check().await.map_err(|_| unreachable_error())? else {
        return Err(CommandError::new(
            "no_update",
            "You already have the newest version.",
        ));
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| {
            CommandError::new(
                "update_failed",
                "The update couldn't be installed. Nothing was changed — you can download it from the releases page instead.",
            )
            .with_detail(e.to_string())
        })?;
    app.restart()
}
