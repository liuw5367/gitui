// 取某个进程名下所有窗口的 CGWindowID 与边界，供 screencapture -l 使用。
//
// 为什么需要它：`screencapture -l<窗口 id>` 能只截一个窗口，但窗口 id 的常规
// 来源是 Apple Events（`osascript` + System Events），它需要「自动化」权限，
// 而 CGWindowListCopyWindowInfo 只需要「屏幕录制」权限，是更窄的一条路。
//
// 用法：
//   swift tools/winid.swift <进程名子串>        # 打印 "id x y w h title"，每行一个
//   swift tools/winid.swift <进程名子串> --id   # 只打印最大的那个窗口 id
//   swift tools/winid.swift <进程名子串> --smallest  # 只打印最小的那个窗口 id
import CoreGraphics
import Foundation

let arguments = CommandLine.arguments
guard arguments.count >= 2 else {
    FileHandle.standardError.write(Data("用法: winid <进程名子串> [--id|--smallest]\n".utf8))
    exit(2)
}
let needle = arguments[1].lowercased()
let onlyId = arguments.contains("--id")
let onlySmallest = arguments.contains("--smallest")

guard let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else {
    FileHandle.standardError.write(Data("取不到窗口列表\n".utf8))
    exit(1)
}

struct Found {
    let id: Int
    let owner: String
    let title: String
    let bounds: CGRect
}

var found: [Found] = []
for window in windows {
    guard let owner = window[kCGWindowOwnerName as String] as? String else { continue }
    let layer = window[kCGWindowLayer as String] as? Int ?? 0
    // 只要普通窗口层（菜单栏、Dock、悬浮层都排除）。
    guard layer == 0 else { continue }
    guard owner.lowercased().contains(needle) else { continue }
    let id = window[kCGWindowNumber as String] as? Int ?? 0
    let title = window[kCGWindowName as String] as? String ?? ""
    let raw = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
    let bounds = CGRect(
        x: raw["X"] as? Double ?? 0,
        y: raw["Y"] as? Double ?? 0,
        width: raw["Width"] as? Double ?? 0,
        height: raw["Height"] as? Double ?? 0
    )
    guard id != 0, bounds.width > 100, bounds.height > 100 else { continue }
    found.append(Found(id: id, owner: owner, title: title, bounds: bounds))
}

if found.isEmpty {
    FileHandle.standardError.write(Data("没有匹配 \"\(needle)\" 的窗口\n".utf8))
    exit(1)
}

if onlyId {
    let biggest = found.max { $0.bounds.width * $0.bounds.height < $1.bounds.width * $1.bounds.height }!
    print(biggest.id)
} else if onlySmallest {
    let smallest = found.min { $0.bounds.width * $0.bounds.height < $1.bounds.width * $1.bounds.height }!
    print(smallest.id)
} else {
    for window in found {
        let b = window.bounds
        print("\(window.id) \(Int(b.origin.x)) \(Int(b.origin.y)) \(Int(b.width)) \(Int(b.height)) \(window.owner) \(window.title)")
    }
}
