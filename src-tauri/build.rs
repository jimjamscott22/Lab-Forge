// Listing commands here switches Tauri from "every registered command is callable"
// to an explicit allow-list: each one needs a permission in capabilities/.
const COMMANDS: &[&str] = &[
    "list_challenges",
    "get_challenge",
    "create_challenge",
    "update_challenge",
    "delete_challenge",
    "start_attempt",
    "get_attempt",
    "complete_step",
    "save_note",
    "get_dashboard",
    "open_external_link",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
