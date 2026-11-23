//! Per-operation AskPass bridge between Git/OpenSSH and the native UI.

use nsh_core::prelude::*;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

const MAX_PROMPT_BYTES: usize = 64 * 1024;
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct AskPassClient {
    sender: Sender<UiEvent>,
}

#[derive(Clone)]
pub(crate) struct AskPassUi {
    pub(crate) open: Signal<bool>,
    active: Signal<Option<PromptState>>,
    queued: Signal<Vec<PromptState>>,
}

#[derive(Clone)]
struct PromptState {
    id: u64,
    prompt: String,
    input: Signal<String>,
    secret: bool,
    reply: SyncSender<Option<String>>,
    cancelled: Arc<AtomicBool>,
}

impl PartialEq for PromptState {
    fn eq(&self, other: &Self) -> bool {
        return self.id == other.id;
    }
}

impl Eq for PromptState {}

pub(crate) enum UiEvent {
    Prompt {
        id: u64,
        prompt: String,
        reply: SyncSender<Option<String>>,
        cancelled: Arc<AtomicBool>,
    },
    Dismiss(u64),
}

pub(crate) fn channel() -> (AskPassUi, AskPassClient, Receiver<UiEvent>) {
    let (sender, receiver) = mpsc::channel();
    return (
        AskPassUi {
            open: signal(false),
            active: signal(None),
            queued: signal(Vec::new()),
        },
        AskPassClient { sender },
        receiver,
    );
}

impl AskPassUi {
    /// Move AskPass requests onto the UI task; helper processes never touch UI signals.
    pub(crate) fn install(&self, receiver: Receiver<UiEvent>) {
        let ui = self.clone();
        spawn(async move {
            loop {
                match receiver.try_recv() {
                    Ok(UiEvent::Prompt {
                        id,
                        prompt,
                        reply,
                        cancelled,
                    }) => {
                        let state = PromptState {
                            id,
                            secret: is_secret_prompt(prompt.as_str()),
                            prompt,
                            input: signal(String::new()),
                            reply,
                            cancelled,
                        };
                        if ui.active.get_untracked().is_none() {
                            ui.active.set(Some(state));
                            ui.open.set(true);
                        } else {
                            let mut queued = ui.queued.get_untracked();
                            queued.push(state);
                            ui.queued.set(queued);
                        }
                    }
                    Ok(UiEvent::Dismiss(id)) => ui.dismiss(id),
                    Err(TryRecvError::Disconnected) => return,
                    Err(TryRecvError::Empty) => {
                        timer(Duration::from_millis(25)).await;
                    }
                }
            }
        });
    }

    pub(crate) fn view(&self, language: Signal<SharedString>) -> impl IntoElement + use<> {
        let ui = self.clone();
        return dynamic(move || -> AnyElement {
            let Some(prompt) = ui.active.get() else {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            };
            let ui_for_content = ui.clone();
            let ui_for_dismiss = ui.clone();
            let prompt_for_content = prompt.clone();
            return crate::ui::dialog::modal(
                || true,
                move || crate::i18n::t(language.get().as_str(), "askpass.title"),
                300,
                move || ui_for_content.prompt_panel(prompt_for_content.clone(), language),
                move || ui_for_dismiss.answer(prompt.id, None),
            )
            .into_element();
        });
    }

    pub(crate) fn dismiss_active(&self) {
        if let Some(prompt) = self.active.get_untracked() {
            self.answer(prompt.id, None);
        }
    }

    fn prompt_panel(&self, prompt: PromptState, language: Signal<SharedString>) -> AnyElement {
        let input = prompt.input;
        let update = input;
        let placeholder_key = if prompt.secret {
            "askpass.secretPlaceholder"
        } else {
            "askpass.usernamePlaceholder"
        };
        let input_element = crate::ui::text_input::askpass_input(
            input,
            move || crate::i18n::t(language.get().as_str(), placeholder_key),
            prompt.secret,
            move |text| update.set(text),
        )
        .into_element();
        let prompt_text = prompt.prompt.clone();
        let answer_ui = self.clone();
        let cancel_ui = self.clone();
        let dismiss_ui = self.clone();
        let id = prompt.id;
        let body = view()
            .w_full()
            .flex_col()
            .gap(px(crate::ui::GAP))
            .child(
                text(move || prompt_text.clone())
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || crate::design::text2()),
            )
            .child(
                text(move || crate::i18n::t(language.get().as_str(), "askpass.hint"))
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || crate::design::text3()),
            )
            .child(input_element);
        let footer = vec![
            crate::ui::button::button(
                move || crate::i18n::t(language.get().as_str(), "common.cancel"),
                crate::ui::button::ButtonVariant::Secondary,
                crate::ui::button::ButtonSize::Regular,
                false,
                move || cancel_ui.answer(id, None),
            )
            .into_element(),
            crate::ui::button::button(
                move || crate::i18n::t(language.get().as_str(), "askpass.submit"),
                crate::ui::button::ButtonVariant::Primary,
                crate::ui::button::ButtonSize::Regular,
                false,
                move || {
                    let answer = input.get_untracked();
                    answer_ui.answer(id, Some(answer));
                },
            )
            .into_element(),
        ];
        return crate::ui::dialog::dialog_panel(
            440.0,
            move || crate::i18n::t(language.get().as_str(), "askpass.title"),
            Some(std::rc::Rc::new(move || {
                crate::i18n::t(language.get().as_str(), "common.close")
            })),
            body,
            footer,
            move || dismiss_ui.answer(id, None),
        );
    }

    fn answer(&self, id: u64, answer: Option<String>) {
        let Some(prompt) = self.active.get_untracked() else {
            return;
        };
        if prompt.id != id {
            return;
        }
        if answer.is_none() {
            prompt.cancelled.store(true, Ordering::Release);
        }
        let _ = prompt.reply.send(answer);
        self.advance();
    }

    fn dismiss(&self, id: u64) {
        if self
            .active
            .get_untracked()
            .is_some_and(|prompt| prompt.id == id)
        {
            self.answer(id, None);
            return;
        }
        let mut queued = self.queued.get_untracked();
        queued.retain(|prompt| prompt.id != id);
        self.queued.set(queued);
    }

    fn advance(&self) {
        let mut queued = self.queued.get_untracked();
        let next = if queued.is_empty() {
            None
        } else {
            Some(queued.remove(0))
        };
        self.queued.set(queued);
        self.active.set(next);
        self.open.set(self.active.get_untracked().is_some());
    }
}

pub(crate) struct AskPassSession {
    executable: git::GitExecutable,
    id: u64,
    stopped: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    directory: PathBuf,
    listener_thread: Option<std::thread::JoinHandle<()>>,
    client: AskPassClient,
}

impl AskPassSession {
    #[cfg(unix)]
    pub(crate) fn new(
        executable: git::GitExecutable,
        client: AskPassClient,
    ) -> std::io::Result<Self> {
        let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let directory = create_private_directory()?;
        let socket_path = directory.join("s");
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&directory);
                return Err(error);
            }
        };
        if let Err(error) = listener.set_nonblocking(true) {
            drop(listener);
            let _ = std::fs::remove_dir_all(&directory);
            return Err(error);
        }
        let stopped = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::new(AtomicBool::new(false));
        let thread_stopped = stopped.clone();
        let thread_cancelled = cancelled.clone();
        let sender = client.sender.clone();
        let listener_thread = match std::thread::Builder::new()
            .name(format!("gitui-askpass-{id}"))
            .spawn(move || {
                while !thread_stopped.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            serve_helper(stream, id, &sender, &thread_stopped, &thread_cancelled)
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            }) {
            Ok(thread) => thread,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&directory);
                return Err(error);
            }
        };
        let current_exe = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                stopped.store(true, Ordering::Release);
                let _ = listener_thread.join();
                let _ = std::fs::remove_dir_all(&directory);
                return Err(error);
            }
        };
        let executable = executable
            .with_env("GITUI_ASKPASS_SOCKET", socket_path.as_os_str())
            .with_env("GIT_ASKPASS", current_exe.as_os_str())
            .with_env("SSH_ASKPASS", current_exe.as_os_str())
            .with_env("SSH_ASKPASS_REQUIRE", "force");
        return Ok(Self {
            executable,
            id,
            stopped,
            cancelled,
            directory,
            listener_thread: Some(listener_thread),
            client,
        });
    }

    #[cfg(not(unix))]
    pub(crate) fn new(
        _executable: git::GitExecutable,
        _client: AskPassClient,
    ) -> std::io::Result<Self> {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "AskPass requires a Unix domain socket",
        ));
    }

    pub(crate) fn executable(&self) -> &git::GitExecutable {
        return &self.executable;
    }

    pub(crate) fn was_cancelled(&self) -> bool {
        return self.cancelled.load(Ordering::Acquire);
    }
}

impl Drop for AskPassSession {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.client.sender.send(UiEvent::Dismiss(self.id));
        if let Some(thread) = self.listener_thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[cfg(unix)]
fn create_private_directory() -> std::io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let base = Path::new("/tmp");
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..64_u64 {
        let path = base.join(format!(
            "gitui-askpass-{}-{since_epoch:x}-{attempt:x}",
            std::process::id()
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => {
                if let Err(error) =
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                {
                    let _ = std::fs::remove_dir(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    return Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not create a private AskPass directory",
    ));
}

#[cfg(unix)]
fn serve_helper(
    mut stream: UnixStream,
    id: u64,
    sender: &Sender<UiEvent>,
    stopped: &AtomicBool,
    cancelled: &AtomicBool,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Ok(prompt) = read_frame(&mut stream) else {
        return;
    };
    let Ok(prompt) = String::from_utf8(prompt) else {
        return;
    };
    let (reply, answer) = mpsc::sync_channel(1);
    let ui_cancelled = Arc::new(AtomicBool::new(false));
    if sender
        .send(UiEvent::Prompt {
            id,
            prompt,
            reply,
            cancelled: ui_cancelled.clone(),
        })
        .is_err()
    {
        let _ = write_response(&mut stream, None);
        return;
    }
    let response = loop {
        if stopped.load(Ordering::Acquire) {
            break None;
        }
        match answer.recv_timeout(Duration::from_millis(100)) {
            Ok(answer) => break answer,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break None,
        }
    };
    if ui_cancelled.load(Ordering::Acquire) {
        cancelled.store(true, Ordering::Release);
    }
    if response.is_none() {
        stopped.store(true, Ordering::Release);
    }
    let _ = write_response(&mut stream, response);
}

#[cfg(unix)]
fn read_frame(stream: &mut UnixStream) -> std::io::Result<Vec<u8>> {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_PROMPT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "AskPass prompt is too long",
        ));
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes)?;
    return Ok(bytes);
}

#[cfg(unix)]
fn write_frame(stream: &mut UnixStream, bytes: &[u8]) -> std::io::Result<()> {
    if bytes.len() > MAX_PROMPT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "AskPass value is too long",
        ));
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(bytes)?;
    return Ok(());
}

#[cfg(unix)]
fn write_response(stream: &mut UnixStream, response: Option<String>) -> std::io::Result<()> {
    match response {
        Some(answer) if answer.len() <= MAX_PROMPT_BYTES => {
            stream.write_all(&[1])?;
            return write_frame(stream, answer.as_bytes());
        }
        _ => {
            stream.write_all(&[0])?;
            stream.write_all(&0_u32.to_be_bytes())?;
            return Ok(());
        }
    }
}

#[cfg(unix)]
fn helper_answer(socket: &Path, prompt: &str) -> std::io::Result<Option<String>> {
    if prompt.len() > MAX_PROMPT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "AskPass prompt is too long",
        ));
    }
    let mut stream = UnixStream::connect(socket)?;
    write_frame(&mut stream, prompt.as_bytes())?;
    let mut status = [0_u8; 1];
    stream.read_exact(&mut status)?;
    let response = read_frame(&mut stream)?;
    if status[0] != 1 {
        return Ok(None);
    }
    return String::from_utf8(response)
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error));
}

/// Return `Some(exit_code)` when this process was launched by Git/OpenSSH as AskPass.
pub fn run_askpass_helper() -> Option<i32> {
    let socket = std::env::var_os("GITUI_ASKPASS_SOCKET")?;
    let mut args = std::env::args_os();
    let _program = args.next();
    let prompt = args.next()?;
    if args.next().is_some() {
        return Some(1);
    }
    #[cfg(unix)]
    {
        let answer = helper_answer(Path::new(&socket), prompt.to_string_lossy().as_ref());
        return match answer {
            Ok(Some(answer)) => {
                let mut stdout = std::io::stdout().lock();
                if stdout.write_all(answer.as_bytes()).is_ok()
                    && stdout.write_all(b"\n").is_ok()
                    && stdout.flush().is_ok()
                {
                    Some(0)
                } else {
                    Some(1)
                }
            }
            Ok(None) | Err(_) => Some(1),
        };
    }
    #[cfg(not(unix))]
    {
        let _ = (socket, prompt);
        return Some(1);
    }
}

fn is_secret_prompt(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    return prompt.contains("password")
        || prompt.contains("passphrase")
        || prompt.contains("token")
        || prompt.contains("secret");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_password_and_ssh_passphrase_prompts_as_secret() {
        assert!(is_secret_prompt("Password for 'https://example.com':"));
        assert!(is_secret_prompt(
            "Enter passphrase for key '/tmp/id_ed25519':"
        ));
        assert!(!is_secret_prompt("Username for 'https://example.com':"));
    }

    #[cfg(unix)]
    #[test]
    fn askpass_session_adds_isolated_helper_environment_and_cleans_up() {
        let (sender, _receiver) = mpsc::channel();
        let client = AskPassClient { sender };
        let base = git::GitExecutable::from_path("/custom/git");
        let session = AskPassSession::new(base, client).expect("start AskPass session");
        let executable = session.executable();
        assert_eq!(executable.path(), Path::new("/custom/git"));
        let command = executable.command();
        assert_eq!(
            command
                .get_envs()
                .find(|(key, _)| *key == "GIT_ASKPASS")
                .unwrap()
                .1,
            Some(std::env::current_exe().unwrap().as_os_str())
        );
        assert_eq!(
            command
                .get_envs()
                .find(|(key, _)| *key == "SSH_ASKPASS")
                .unwrap()
                .1,
            Some(std::env::current_exe().unwrap().as_os_str())
        );
        assert_eq!(
            command
                .get_envs()
                .find(|(key, _)| *key == "SSH_ASKPASS_REQUIRE")
                .unwrap()
                .1,
            Some(std::ffi::OsStr::new("force"))
        );
        let directory = session.directory.clone();
        let metadata = std::fs::metadata(&directory).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
        drop(session);
        assert!(!directory.exists());
    }

    #[cfg(unix)]
    #[test]
    fn helper_returns_the_ui_answer_and_reports_user_cancellation() {
        let (sender, receiver) = mpsc::channel();
        let client = AskPassClient { sender };
        let session =
            AskPassSession::new(git::GitExecutable::system(), client).expect("start AskPass");
        let command = session.executable().command();
        let socket = command
            .get_envs()
            .find(|(key, _)| *key == "GITUI_ASKPASS_SOCKET")
            .and_then(|(_, value)| value)
            .map(PathBuf::from)
            .expect("AskPass socket environment");

        let helper_socket = socket.clone();
        let helper = std::thread::spawn(move || {
            helper_answer(&helper_socket, "Password for 'https://example.invalid':")
        });
        let UiEvent::Prompt { prompt, reply, .. } = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("receive AskPass request")
        else {
            panic!("expected AskPass prompt");
        };
        assert!(prompt.starts_with("Password for"));
        reply.send(Some("ephemeral-secret".to_string())).unwrap();
        assert_eq!(
            helper.join().unwrap().unwrap().as_deref(),
            Some("ephemeral-secret")
        );
        assert!(!session.was_cancelled());

        let helper = std::thread::spawn(move || {
            helper_answer(&socket, "Username for 'https://example.invalid':")
        });
        let UiEvent::Prompt {
            reply, cancelled, ..
        } = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("receive second AskPass request")
        else {
            panic!("expected AskPass prompt");
        };
        cancelled.store(true, Ordering::Release);
        reply.send(None).unwrap();
        assert_eq!(helper.join().unwrap().unwrap(), None);
        assert!(session.was_cancelled());
    }
}
