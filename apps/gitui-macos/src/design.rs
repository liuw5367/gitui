//! Design tokens from the canvas CSS (light and dark).

use nsh_base::theme as base_theme;
use nsh_core::prelude::*;

pub(crate) const THEME_JSON: &str = r##"{
  "cssVars": {
    "light": {
      "background": "#FFFFFF",
      "foreground": "#1D1D1F",
      "card": "#F7F7F9",
      "card-foreground": "#1D1D1F",
      "popover": "#FCFCFD",
      "popover-foreground": "#1D1D1F",
      "primary": "#007AFF",
      "primary-foreground": "#FFFFFF",
      "secondary": "#F1F1F4",
      "secondary-foreground": "#1D1D1F",
      "muted": "#F1F1F4",
      "muted-foreground": "#6E6E76",
      "accent": "#E7F0FF",
      "accent-foreground": "#1D1D1F",
      "destructive": "#B3261E",
      "border": "#E2E2E6",
      "input": "#D2D2D8",
      "ring": "#007AFF",
      "sidebar": "#F4F4F6",
      "sidebar-foreground": "#1D1D1F",
      "sidebar-border": "#E2E2E6",
      "titlebar": "#F0F0F2",
      "toolbar": "#F5F5F7",
      "statusbar": "#F7F7F8",
      "panel": "#F7F7F9",
      "panel2": "#F1F1F4",
      "linesoft": "#ECECEF",
      "text2": "#5D5D65",
      "text3": "#6E6E76",
      "sel": "#E7F0FF",
      "accentsoft": "#E8F1FF",
      "branchbg": "#E8F1FF",
      "branchfg": "#0B5FCC",
      "tagbg": "#FFF3DD",
      "tagfg": "#835700",
      "remotefg": "#5D5D65",
      "addbg": "#EAF7EF",
      "addfg": "#1B7442",
      "delbg": "#FDEEEC",
      "delfg": "#AE241C",
      "warn": "#A86400",
      "ok": "#1F7A45",
      "danger": "#B3261E"
    },
    "dark": {
      "background": "#1B1B1D",
      "foreground": "#F2F2F4",
      "card": "#1F1F21",
      "card-foreground": "#F2F2F4",
      "popover": "#2C2C2E",
      "popover-foreground": "#F2F2F4",
      "primary": "#0A84FF",
      "primary-foreground": "#FFFFFF",
      "secondary": "#262628",
      "secondary-foreground": "#F2F2F4",
      "muted": "#262628",
      "muted-foreground": "#8E8E96",
      "accent": "#15324F",
      "accent-foreground": "#F2F2F4",
      "destructive": "#FF8A80",
      "border": "#343438",
      "input": "#3C3C42",
      "ring": "#0A84FF",
      "sidebar": "#232325",
      "sidebar-foreground": "#F2F2F4",
      "sidebar-border": "#343438",
      "titlebar": "#262628",
      "toolbar": "#2A2A2C",
      "statusbar": "#242426",
      "panel": "#1F1F21",
      "panel2": "#262628",
      "linesoft": "#2C2C30",
      "text2": "#B3B3BB",
      "text3": "#8E8E96",
      "sel": "#15324F",
      "accentsoft": "#15324F",
      "branchbg": "#15324F",
      "branchfg": "#8CC2FF",
      "tagbg": "#3A2E16",
      "tagfg": "#F0C674",
      "remotefg": "#A9A9B1",
      "addbg": "#12281B",
      "addfg": "#7EE2A8",
      "delbg": "#2B1615",
      "delfg": "#FF9B93",
      "warn": "#E8B04A",
      "ok": "#6FD39A",
      "danger": "#FF8A80"
    }
  }
}"##;

pub(crate) fn token(name: &str) -> AdaptiveColor {
    let theme = base_theme();
    theme.color(name).unwrap_or(theme.foreground)
}


pub(crate) fn text() -> AdaptiveColor {
    base_theme().foreground
}


pub(crate) fn titlebar() -> AdaptiveColor {
    token("titlebar")
}

pub(crate) fn toolbar() -> AdaptiveColor {
    token("toolbar")
}

pub(crate) fn statusbar() -> AdaptiveColor {
    token("statusbar")
}

pub(crate) fn sidebar() -> AdaptiveColor {
    base_theme().sidebar
}

pub(crate) fn primary() -> AdaptiveColor {
    base_theme().primary
}

pub(crate) fn primary_foreground() -> AdaptiveColor {
    base_theme().primary_foreground
}

pub(crate) fn danger() -> AdaptiveColor {
    base_theme().destructive
}

pub(crate) fn warn() -> AdaptiveColor {
    token("warn")
}

pub(crate) fn line() -> AdaptiveColor {
    base_theme().border
}


pub(crate) fn text2() -> AdaptiveColor {
    token("text2")
}

pub(crate) fn text3() -> AdaptiveColor {
    token("text3")
}





