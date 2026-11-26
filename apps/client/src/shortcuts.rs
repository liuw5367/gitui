//! 快捷键注册表（手册 1.3、设计板 25）。
//!
//! 菜单、设置页的快捷键列表与按键分发共用这一份数据：改一处三处一起变。
//! nsh 0.6.0 的 `on_key` 只能在挂载时静态绑定，无法在运行时解绑，因此可绑定的
//! 组合是一份**固定清单**（[`catalog`]）：应用把清单里的每个组合都绑一次，按下时
//! 再查表决定执行哪个动作。录制因此只能在清单内改键，界面会写明这一点。
//!
//! 清单刻意排除 macOS 保留键（⌘Q / ⌘H / ⌘M / ⌘⇧H）与 Close All Windows 的 ⌥⌘W：
//! `on_key` 遇到系统保留键会 panic，关闭窗口组合由原生菜单处理。

use crate::i18n;

/// 动作所属的分组（设置页 25 的 Global / Repository）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    Repository,
}

/// 一个可绑定动作。
#[derive(Clone, Copy, Debug)]
pub struct Action {
    /// 稳定标识，同时也是命令面板与菜单用的命令 id。
    pub id: &'static str,
    /// 文案键；`tab1`…`tab9` 使用带 `%{n}` 的同一个键。
    pub key: &'static str,
    /// 说明文案键。
    pub desc: &'static str,
    /// 默认组合（解析规则见 `nsh/keymap`）。
    pub default: &'static str,
    /// 设置页的分组。
    pub scope: Scope,
}

/// 全部可绑定动作，顺序即设置页的显示顺序。
pub const ACTIONS: [Action; 23] = [
    Action {
        id: "open",
        key: "shortcut.openRepo",
        desc: "shortcut.descOpen",
        default: "mod+o",
        scope: Scope::Global,
    },
    Action {
        id: "openPalette",
        key: "shortcut.palette",
        desc: "shortcut.descPalette",
        default: "mod+p",
        scope: Scope::Global,
    },
    // 新建标签页：打开 Welcome 模态层挑选仓库，也为键盘用户提供入口。
    Action {
        id: "newTab",
        key: "shortcut.newTab",
        desc: "shortcut.descNewTab",
        default: "mod+t",
        scope: Scope::Global,
    },
    Action {
        id: "closeTab",
        key: "shortcut.closeTab",
        desc: "shortcut.descCloseTab",
        default: "mod+w",
        scope: Scope::Global,
    },
    Action {
        id: "refresh",
        key: "shortcut.refresh",
        desc: "shortcut.descRefresh",
        default: "mod+r",
        scope: Scope::Global,
    },
    Action {
        id: "filter",
        key: "shortcut.filter",
        desc: "shortcut.descFilter",
        default: "mod+f",
        scope: Scope::Global,
    },
    Action {
        id: "filterHistory",
        key: "shortcut.filterHistory",
        desc: "shortcut.descFilterHistory",
        default: "mod+shift+f",
        scope: Scope::Global,
    },
    Action {
        id: "push",
        key: "shortcut.push",
        desc: "shortcut.descPush",
        default: "mod+shift+p",
        scope: Scope::Repository,
    },
    Action {
        id: "commit",
        key: "shortcut.commit",
        desc: "shortcut.descCommit",
        default: "mod+shift+k",
        scope: Scope::Repository,
    },
    Action {
        id: "primary",
        key: "shortcut.primaryAction",
        desc: "shortcut.descPrimary",
        default: "mod+enter",
        scope: Scope::Repository,
    },
    Action {
        id: "settings",
        key: "shortcut.settings",
        desc: "shortcut.descSettings",
        default: "mod+,",
        scope: Scope::Global,
    },
    Action {
        id: "toggleTheme",
        key: "shortcut.cycleAppearance",
        desc: "shortcut.descTheme",
        default: "mod+shift+a",
        scope: Scope::Global,
    },
    Action {
        id: "prevTab",
        key: "shortcut.prevTab",
        desc: "shortcut.descPrevTab",
        default: "mod+shift+[",
        scope: Scope::Global,
    },
    Action {
        id: "nextTab",
        key: "shortcut.nextTab",
        desc: "shortcut.descNextTab",
        default: "mod+shift+]",
        scope: Scope::Global,
    },
    Action {
        id: "tab1",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+1",
        scope: Scope::Global,
    },
    Action {
        id: "tab2",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+2",
        scope: Scope::Global,
    },
    Action {
        id: "tab3",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+3",
        scope: Scope::Global,
    },
    Action {
        id: "tab4",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+4",
        scope: Scope::Global,
    },
    Action {
        id: "tab5",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+5",
        scope: Scope::Global,
    },
    Action {
        id: "tab6",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+6",
        scope: Scope::Global,
    },
    Action {
        id: "tab7",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+7",
        scope: Scope::Global,
    },
    Action {
        id: "tab8",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+8",
        scope: Scope::Global,
    },
    Action {
        id: "tab9",
        key: "shortcut.tab",
        desc: "shortcut.descTab",
        default: "mod+9",
        scope: Scope::Global,
    },
];

/// 可绑定的字母：去掉 macOS 保留的 `q`（退出）、`h`（隐藏）、`m`（最小化）。
const LETTERS: [&str; 23] = [
    "a", "b", "c", "d", "e", "f", "g", "i", "j", "k", "l", "n", "o", "p", "r", "s", "t", "u", "v",
    "w", "x", "y", "z",
];

/// 可绑定的组合清单：⌘+字母、⇧⌘+字母、⌘+1…9、⌘⇧[ ]、⌘↵、⌘,。
pub fn catalog() -> Vec<String> {
    let mut combos: Vec<String> = Vec::new();
    for letter in LETTERS {
        combos.push(format!("mod+{letter}"));
    }
    for letter in LETTERS {
        combos.push(format!("mod+shift+{letter}"));
    }
    for digit in 1..=9 {
        combos.push(format!("mod+{digit}"));
    }
    combos.push("mod+shift+[".to_string());
    combos.push("mod+shift+]".to_string());
    combos.push("mod+enter".to_string());
    combos.push("mod+,".to_string());
    // Close All Windows owns this native menu shortcut; it must not become user-rebindable.
    combos.retain(|combo| combo != "alt+mod+w");
    return combos;
}

/// 录制时要回显的组合：可绑定清单之外的常见按法（⌥/⌃ 系列、⌘⇧ 数字）。
///
/// 这些组合绑定不上，但录制时按下它们要能立刻看到并按到的是哪几个键，
/// 并得到「不可绑定」的说明；否则按下去毫无反馈，像是录制没生效。
pub fn observable() -> Vec<String> {
    let mut combos: Vec<String> = Vec::new();
    let keys: Vec<String> = LETTERS
        .iter()
        .map(|letter| (*letter).to_string())
        .chain((1..=9).map(|digit| digit.to_string()))
        .collect();
    for mask in ["alt", "alt+shift", "ctrl", "ctrl+shift"] {
        for key in &keys {
            combos.push(format!("{mask}+{key}"));
        }
    }
    for digit in 1..=9 {
        combos.push(format!("mod+shift+{digit}"));
    }
    combos.retain(|combo| !catalog().contains(combo));
    return combos;
}

/// 改键失败的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindError {
    /// 这个组合不在可绑定清单里（录制到了未绑定过的键）。
    NotBindable,
    /// 这个组合已被另一个动作占用；携带对方的 id。
    Conflict(&'static str),
}

/// 当前生效的绑定：与 [`ACTIONS`] 同序的组合字符串。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registry {
    combos: Vec<String>,
}

impl Default for Registry {
    fn default() -> Self {
        return Registry::defaults();
    }
}

impl Registry {
    /// 默认绑定。
    pub fn defaults() -> Registry {
        return Registry {
            combos: ACTIONS
                .iter()
                .map(|action| action.default.to_string())
                .collect(),
        };
    }

    /// 读出 `overrides`（只存与默认值不同的项）并叠加到默认值上。
    ///
    /// 未知动作与非法组合直接忽略：旧版本留下的键不能把应用带进无效状态。
    pub fn with_overrides(overrides: &[(String, String)]) -> Registry {
        let mut registry = Registry::defaults();
        for (id, combo) in overrides {
            let Some(index) = ACTIONS.iter().position(|action| action.id == id.as_str()) else {
                continue;
            };
            if combo.is_empty() {
                registry.combos[index].clear();
                continue;
            }
            if !catalog().iter().any(|known| known == combo) {
                continue;
            }
            if let Some(taken) = registry.combo_owner(combo)
                && taken != index
            {
                continue;
            }
            registry.combos[index] = combo.clone();
        }
        return registry;
    }

    /// 某动作当前的组合；未知 id 返回空串。
    pub fn combo(&self, id: &str) -> &str {
        return ACTIONS
            .iter()
            .position(|action| action.id == id)
            .and_then(|index| self.combos.get(index))
            .map(|combo| combo.as_str())
            .unwrap_or("");
    }

    /// 某个组合当前归谁；空组合不属于任何动作。
    pub fn action_for(&self, combo: &str) -> Option<&'static str> {
        if combo.is_empty() {
            return None;
        }
        return self.combo_owner(combo).map(|index| ACTIONS[index].id);
    }

    /// 解绑一个动作：行内显示 `Unassigned`，再录制可重新绑定。
    pub fn clear(&mut self, id: &'static str) {
        if let Some(index) = ACTIONS.iter().position(|action| action.id == id) {
            self.combos[index].clear();
        }
    }

    /// 改键：组合必须可绑定且未被其它动作占用。
    pub fn rebind(&mut self, id: &'static str, combo: &str) -> Result<(), BindError> {
        let Some(index) = ACTIONS.iter().position(|action| action.id == id) else {
            return Err(BindError::NotBindable);
        };
        if !catalog().iter().any(|known| known == combo) {
            return Err(BindError::NotBindable);
        }
        if let Some(taken) = self.combo_owner(combo)
            && taken != index
        {
            return Err(BindError::Conflict(ACTIONS[taken].id));
        }
        self.combos[index] = combo.to_string();
        return Ok(());
    }

    /// 恢复全部默认值。
    pub fn restore_defaults(&mut self) {
        *self = Registry::defaults();
    }

    /// 需要持久化的项：与默认值不同的 `(id, combo)`。
    pub fn overrides(&self) -> Vec<(String, String)> {
        return ACTIONS
            .iter()
            .enumerate()
            .filter(|(index, action)| self.combos[*index] != action.default)
            .map(|(index, action)| (action.id.to_string(), self.combos[index].clone()))
            .collect();
    }

    /// 某个组合的下标（没有则为 `None`）。
    fn combo_owner(&self, combo: &str) -> Option<usize> {
        if combo.is_empty() {
            return None;
        }
        return self.combos.iter().position(|known| known == combo);
    }
}

/// 动作在界面上的名字（`tab1`…`tab9` 用同一个带 `%{n}` 的键）。
pub fn label(action: &Action, language: &str) -> String {
    if let Some(number) = action.id.strip_prefix("tab") {
        return i18n::with_arguments(language, action.key, &[("n", number.to_string())]);
    }
    return i18n::t(language, action.key);
}

/// 把组合拆成逐枚键帽：`mod+shift+k` → `["⌘", "⇧", "K"]`。
pub fn keycaps(combo: &str) -> Vec<String> {
    if combo.is_empty() {
        return Vec::new();
    }
    let mut modifiers: Vec<&str> = Vec::new();
    let mut key = "";
    for part in combo.split('+') {
        match part {
            "mod" => modifiers.push("⌘"),
            "shift" => modifiers.push("⇧"),
            "alt" => modifiers.push("⌥"),
            "ctrl" => modifiers.push("⌃"),
            "enter" => key = "↵",
            "," => key = ",",
            other => key = other,
        }
    }
    let mut caps: Vec<String> = Vec::new();
    for symbol in ["⌃", "⌥", "⇧", "⌘"] {
        if modifiers.contains(&symbol) {
            caps.push(symbol.to_string());
        }
    }
    let mut letters = key.chars();
    let shown = match (letters.next(), letters.next()) {
        (Some(first), None) if first.is_ascii_alphabetic() => {
            first.to_ascii_uppercase().to_string()
        }
        _ => key.to_string(),
    };
    if !shown.is_empty() {
        caps.push(shown);
    }
    return caps;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observable_combos_are_parseable_and_not_reserved() {
        // 这些组合在设置窗口挂载时逐个 `on_key`：任何一个解析失败或是系统保留键
        // 都会在打开设置页时 panic，所以在这里逐个过一遍。
        let combos = observable();
        assert!(!combos.is_empty(), "录制回显的组合不应该是空的");
        for combo in &combos {
            let parsed = nsh_core::keymap::Keystroke::parse_with(combo, true)
                .unwrap_or_else(|error| panic!("{combo}: {error}"));
            assert_eq!(
                parsed.reserved(nsh_core::platform::Os::MacOs),
                None,
                "{combo} 是系统保留键，不能绑定"
            );
        }
        assert!(
            combos.iter().all(|combo| !catalog().contains(combo)),
            "可绑定清单里的组合由 catalog 自己绑定，不该重复出现在回显集合里"
        );
    }

    #[test]
    fn close_all_windows_shortcut_is_not_bindable() {
        assert!(!catalog().iter().any(|combo| combo == "alt+mod+w"));
        assert_eq!(
            Registry::defaults().rebind("closeTab", "alt+mod+w"),
            Err(BindError::NotBindable)
        );
    }

    #[test]
    fn defaults_are_unique_and_bindable() {
        let registry = Registry::defaults();
        let mut seen: Vec<&str> = Vec::new();
        for action in ACTIONS.iter() {
            let combo = registry.combo(action.id);
            assert!(
                catalog().iter().any(|known| known == combo),
                "{} 的默认组合不在清单里：{combo}",
                action.id
            );
            assert!(!seen.contains(&combo), "默认组合重复：{combo}");
            assert_eq!(registry.action_for(combo), Some(action.id));
            seen.push(combo);
        }
    }

    #[test]
    fn rebind_changes_dispatch_and_conflicts_are_rejected() {
        let mut registry = Registry::defaults();
        assert_eq!(registry.rebind("commit", "mod+shift+b"), Ok(()));
        assert_eq!(registry.action_for("mod+shift+b"), Some("commit"));
        // 旧组合被释放。
        assert_eq!(registry.action_for("mod+shift+k"), None);
        // 已被占用的组合不能抢。
        assert_eq!(
            registry.rebind("push", "mod+shift+b"),
            Err(BindError::Conflict("commit"))
        );
        // 重新绑回自己不算冲突。
        assert_eq!(registry.rebind("commit", "mod+shift+b"), Ok(()));
    }

    #[test]
    fn only_catalog_combos_can_be_bound() {
        let mut registry = Registry::defaults();
        assert_eq!(
            registry.rebind("commit", "mod+shift+1"),
            Err(BindError::NotBindable)
        );
        // macOS 保留键不在清单里，避免 on_key panic。
        assert!(!catalog().iter().any(|combo| combo == "mod+q"));
        assert!(!catalog().iter().any(|combo| combo == "mod+h"));
        assert!(!catalog().iter().any(|combo| combo == "mod+m"));
    }

    #[test]
    fn overrides_round_trip_and_restore_defaults() {
        let mut registry = Registry::defaults();
        assert!(registry.overrides().is_empty());
        assert_eq!(registry.rebind("push", "mod+shift+e"), Ok(()));
        let overrides = registry.overrides();
        assert_eq!(
            overrides,
            vec![("push".to_string(), "mod+shift+e".to_string())]
        );
        let loaded = Registry::with_overrides(&overrides);
        assert_eq!(loaded.combo("push"), "mod+shift+e");
        // 非法与未知的旧记录被忽略，不能把应用带进无效状态。
        let junk = vec![
            ("nope".to_string(), "mod+shift+e".to_string()),
            ("push".to_string(), "mod+shift+1".to_string()),
            ("push".to_string(), "mod+o".to_string()),
        ];
        let loaded = Registry::with_overrides(&junk);
        assert_eq!(loaded.combo("push"), "mod+shift+p");
        assert_eq!(loaded.combo("open"), "mod+o");
        let mut registry = loaded;
        registry.restore_defaults();
        assert_eq!(registry, Registry::defaults());
    }

    #[test]
    fn clearing_unbinds_and_persists_an_empty_combo() {
        let mut registry = Registry::defaults();
        registry.clear("push");
        assert_eq!(registry.combo("push"), "");
        assert_eq!(registry.action_for(""), None);
        let overrides = registry.overrides();
        assert!(overrides.contains(&("push".to_string(), String::new())));
        let loaded = Registry::with_overrides(&overrides);
        assert_eq!(loaded.combo("push"), "");
        // 重新录制可以恢复。
        assert_eq!(loaded.clone().rebind("push", "mod+shift+e"), Ok(()));
    }

    #[test]
    fn keycaps_split_modifiers_and_keys() {
        assert_eq!(keycaps("mod+shift+k"), vec!["⇧", "⌘", "K"]);
        assert_eq!(keycaps("mod+enter"), vec!["⌘", "↵"]);
        assert_eq!(keycaps("mod+1"), vec!["⌘", "1"]);
        assert_eq!(keycaps(""), Vec::<String>::new());
        // 录制中回显的是任意按法（不限于可绑定清单），⌥⇧ 这类组合也要画全。
        assert_eq!(keycaps("alt+shift+k"), vec!["⌥", "⇧", "K"]);
        assert_eq!(keycaps("ctrl+alt+j"), vec!["⌃", "⌥", "J"]);
    }

    #[test]
    fn tab_labels_expand_the_number() {
        let tab = ACTIONS
            .iter()
            .find(|action| action.id == "tab3")
            .expect("tab3 存在");
        assert_eq!(label(tab, "en"), "Switch to tab 3");
        assert_eq!(label(tab, "zh-Hans"), "切换到标签页 3");
    }
}
