//! SSH key management use cases for the Credentials settings page.

use super::commands::SettingsCommands;
use crate::i18n;
use nsh_core::prelude::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

fn spawn_worker<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> Receiver<T> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(job());
    });
    return receiver;
}

async fn wait_worker<T: Send + 'static>(receiver: Receiver<T>) -> Option<T> {
    loop {
        match receiver.try_recv() {
            Ok(result) => return Some(result),
            Err(TryRecvError::Disconnected) => return None,
            Err(TryRecvError::Empty) => timer(Duration::from_millis(25)).await,
        }
    }
}

impl SettingsCommands {
    /// Copy the `.pub` text after revalidating that the path is an ordinary file in `~/.ssh`.
    pub(crate) fn copy_ssh_public_key(&self, public_name: String) {
        match super::ssh::read_public_key(public_name.as_str()) {
            Ok(text) => crate::shared::os::copy_text(self.notice, self.language, text),
            Err(error) => self.notify_ssh_error(error),
        }
    }

    pub(crate) fn add_ssh_key(&self) {
        if self.state.ssh_operation.get_untracked() {
            return;
        }
        self.state.ssh_operation.set(true);
        let pending = pick_paths(nsh_core::file_dialog::OpenDialog::files());
        let commands = self.clone();
        spawn(async move {
            let source = match pending.await {
                Ok(Some(paths)) => paths.into_iter().next(),
                _ => None,
            };
            let Some(source) = source else {
                commands.state.ssh_operation.set(false);
                return;
            };
            let directory = match super::ssh::ssh_dir() {
                Ok(directory) => directory,
                Err(error) => {
                    commands.state.ssh_operation.set(false);
                    commands.notify_ssh_error(error);
                    return;
                }
            };
            let askpass = commands.askpass.clone();
            let receiver =
                spawn_worker(move || super::ssh::import_key(&source, &directory, askpass));
            let result = wait_worker(receiver)
                .await
                .unwrap_or(Err(super::ssh::SshError::FileOperation));
            commands.state.ssh_operation.set(false);
            match result {
                Ok(_) => {
                    commands.refresh_ssh_keys();
                    commands.notify_ssh_key_result("settings.sshKeyImported");
                }
                Err(error) => commands.notify_ssh_error(error),
            }
        });
    }

    pub(crate) fn open_ssh_generate(&self) {
        if self.state.ssh_operation.get_untracked()
            || self.state.ssh_generate.get_untracked().is_some()
        {
            return;
        }
        let keys = self.state.ssh_keys.get_untracked();
        let mut name = "id_ed25519".to_string();
        let mut suffix = 1_u32;
        while keys
            .iter()
            .any(|key| key.name == name || key.public_name == format!("{name}.pub"))
        {
            name = format!("id_ed25519_{suffix}");
            suffix += 1;
        }
        self.state
            .ssh_generate
            .set(Some(super::state::SshGenerateForm {
                name: signal(name),
                comment: signal(String::new()),
                error: signal(None),
            }));
    }

    pub(crate) fn close_ssh_generate(&self) {
        if !self.state.ssh_operation.get_untracked() {
            self.state.ssh_generate.set(None);
        }
    }

    pub(crate) fn submit_ssh_generate(&self) {
        if self.state.ssh_operation.get_untracked() {
            return;
        }
        let Some(form) = self.state.ssh_generate.get_untracked() else {
            return;
        };
        let name = form.name.get_untracked();
        let comment = form.comment.get_untracked();
        form.error.set(None);
        let language = self.language.get_untracked();
        let Some(directory) = super::ssh::ssh_dir().ok() else {
            form.error.set(Some(i18n::t(
                language.as_str(),
                super::ssh::SshError::NoHome.message_key(),
            )));
            return;
        };
        self.state.ssh_operation.set(true);
        let askpass = self.askpass.clone();
        let receiver = spawn_worker(move || {
            super::ssh::generate_key(name.as_str(), comment.as_str(), &directory, askpass)
        });
        let commands = self.clone();
        spawn(async move {
            let result = wait_worker(receiver)
                .await
                .unwrap_or(Err(super::ssh::SshError::FileOperation));
            commands.state.ssh_operation.set(false);
            match result {
                Ok(_) => {
                    commands.state.ssh_generate.set(None);
                    commands.refresh_ssh_keys();
                    commands.notify_ssh_key_result("settings.sshKeyGenerated");
                }
                Err(error) => form.error.set(Some(i18n::t(
                    commands.language.get_untracked().as_str(),
                    error.message_key(),
                ))),
            }
        });
    }

    pub(crate) fn remove_ssh_key(&self, entry: super::ssh::SshKeyEntry) {
        let Some(name) = entry.private_name else {
            return;
        };
        let expected_fingerprint = entry.fingerprint;
        if self.state.ssh_operation.get_untracked() {
            return;
        }
        self.state.ssh_operation.set(true);
        let language = self.language.get_untracked();
        let message = i18n::with_arguments(
            language.as_str(),
            "settings.sshRemoveMessage",
            &[("name", name.clone())],
        );
        let pending = crate::interactions::confirm(
            i18n::t(language.as_str(), "settings.sshRemoveTitle"),
            message,
            "removeSshKey",
            i18n::t(language.as_str(), "settings.sshRemoveConfirm"),
            i18n::t(language.as_str(), "common.cancel"),
            true,
        );
        let commands = self.clone();
        spawn(async move {
            if !pending.await {
                commands.state.ssh_operation.set(false);
                return;
            }
            let directory = match super::ssh::ssh_dir() {
                Ok(directory) => directory,
                Err(error) => {
                    commands.state.ssh_operation.set(false);
                    commands.notify_ssh_error(error);
                    return;
                }
            };
            let receiver = spawn_worker(move || {
                super::ssh::remove_key(&directory, name.as_str(), expected_fingerprint.as_str())
            });
            let result = wait_worker(receiver)
                .await
                .unwrap_or(Err(super::ssh::SshError::FileOperation));
            commands.state.ssh_operation.set(false);
            match result {
                Ok(()) => {
                    commands.refresh_ssh_keys();
                    commands.notify_ssh_key_result("settings.sshKeyRemoved");
                }
                Err(error) => commands.notify_ssh_error(error),
            }
        });
    }

    fn refresh_ssh_keys(&self) {
        self.state.ssh_keys.set(super::ssh::read_keys());
    }

    fn notify_ssh_error(&self, error: super::ssh::SshError) {
        crate::interactions::notify(
            self.notice,
            i18n::t(self.language.get_untracked().as_str(), error.message_key()),
        );
    }

    fn notify_ssh_key_result(&self, key: &'static str) {
        crate::interactions::notify(
            self.notice,
            i18n::t(self.language.get_untracked().as_str(), key),
        );
    }
}
