//! Read-only SSH public key actions for the Credentials settings page.

use super::commands::SettingsCommands;
use crate::i18n;

impl SettingsCommands {
    /// Copy the `.pub` text after revalidating that the path is an ordinary file in `~/.ssh`.
    pub(crate) fn copy_ssh_public_key(&self, public_name: String) {
        match super::ssh::read_public_key(public_name.as_str()) {
            Ok(text) => crate::shared::os::copy_text(self.notice, self.language, text),
            Err(error) => self.notify_ssh_error(error),
        }
    }

    fn notify_ssh_error(&self, error: super::ssh::SshError) {
        crate::interactions::notify(
            self.notice,
            i18n::t(self.language.get_untracked().as_str(), error.message_key()),
        );
    }
}
