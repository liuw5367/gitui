//! 在原生后端运行应用；`nsh run` 使用同一份根组件。
//!
//! 同一个 Rust crate 由 NSH 按目标平台构建：当前接入 macOS（AppKit 后端），
//! 其他平台在后续阶段接入，界面与状态代码不变。

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;

#[cfg(target_os = "macos")]
fn main() -> impl std::process::Termination {
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<nsh_app::App>()),
        platform::AppOptions {
            title: nsh_app::brand::PRODUCT_NAME.to_owned(),
            // 设计板内容区：1440×900 视口减四边 24px 桌面留白。
            size: (1392, 852),
            ..Default::default()
        },
    )
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "{} 当前只接入 macOS 宿主，其他平台在后续阶段接入",
        nsh_app::brand::PRODUCT_NAME
    );
}
