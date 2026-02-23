use zed_extension_api as zed;

struct DarcyExtension;

impl DarcyExtension {
    fn command_for(worktree: &zed::Worktree) -> String {
        worktree
            .which("darcy-lsp")
            .unwrap_or_else(|| "/Users/jagtesh/.cargo/bin/darcy-lsp".to_string())
    }
}

impl zed::Extension for DarcyExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        if language_server_id.as_ref() != "darcy-lsp" {
            return Err(format!("unknown language server: {}", language_server_id.as_ref()));
        }

        Ok(zed::Command {
            command: Self::command_for(worktree),
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(DarcyExtension);
