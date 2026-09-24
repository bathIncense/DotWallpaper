import SwiftUI
import AppKit

@main
struct DotWallpaperApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 800, minHeight: 480)
        }

        Settings {
            NativeSettingsView()
        }
    }
}

/// Native macOS Settings is reachable through the app menu (Command-,) even
/// when the WebView window is closed and the wallpaper keeps running.
private struct NativeSettingsView: View {
    @State private var libraryDir = ""
    @State private var errorMessage: String?
    @State private var choosing = false

    private var configDirectory: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/com.dot.wallpaper", isDirectory: true)
    }

    private var cacheDirectory: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Caches/com.dot.wallpaper", isDirectory: true)
    }

    var body: some View {
        Form {
            Section("壁纸目录") {
                Text(libraryDir.isEmpty ? "尚未选择" : libraryDir)
                    .font(.callout)
                    .textSelection(.enabled)
                Button("选择或重新授权文件夹…") {
                    guard !choosing else { return }
                    choosing = true
                    DotWallpaperCore.shared.pickLibraryDirectory { path, error in
                        choosing = false
                        if let error {
                            errorMessage = error
                        } else if let path {
                            libraryDir = path
                            NotificationCenter.default.post(name: .dotWallpaperSettingsChanged, object: nil)
                        }
                    }
                }
                .disabled(choosing)
                Text("仅扫描选定目录中的图片与视频；更改目录不会删除文件。")
                    .foregroundStyle(.secondary)
            }

            Section("文件访问权限") {
                Text("通过 macOS 文件选择器授予所选目录访问。若目录无法读取，请重新选择该目录；无需屏幕录制或辅助功能权限。")
                    .foregroundStyle(.secondary)
            }

            Section("本机数据") {
                LabeledContent("配置", value: configDirectory.path)
                Button("在 Finder 中显示配置") {
                    reveal(configDirectory.appendingPathComponent("settings.json"))
                }
                LabeledContent("缓存", value: cacheDirectory.path)
                Button("在 Finder 中显示缓存") {
                    reveal(cacheDirectory)
                }
            }
        }
        .formStyle(.grouped)
        .frame(minWidth: 540, minHeight: 410)
        .onAppear(perform: refresh)
        .alert("无法完成设置", isPresented: Binding(
            get: { errorMessage != nil },
            set: { if !$0 { errorMessage = nil } }
        )) {
            Button("知道了", role: .cancel) { errorMessage = nil }
        } message: {
            Text(errorMessage ?? "请重试")
        }
    }

    private func refresh() {
        guard let json = DotWallpaperCore.shared.getAppSnapshot(),
              let data = json.data(using: .utf8),
              let snapshot = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return }
        libraryDir = snapshot["libraryDir"] as? String ?? ""
    }

    private func reveal(_ url: URL) {
        let target = FileManager.default.fileExists(atPath: url.path) ? url : url.deletingLastPathComponent()
        NSWorkspace.shared.activateFileViewerSelecting([target])
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    var statusItem: NSStatusItem?
    let core = DotWallpaperCore.shared

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Setup menu bar
        setupStatusBar()

        // Setup close-to-hide
        NSApp.setActivationPolicy(.regular)

        // Start core
        core.startup()
    }

    func applicationWillTerminate(_ notification: Notification) {
        core.shutdown()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag {
            for window in NSApp.windows {
                window.makeKeyAndOrderFront(self)
            }
        }
        return true
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return false
    }

    private func setupStatusBar() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)

        if let button = statusItem?.button {
            button.image = NSImage(systemSymbolName: "photo.on.rectangle.angled", accessibilityDescription: "DotWallpaper")
        }

        let menu = NSMenu()

        let openItem = NSMenuItem(title: "打开 DotWallpaper", action: #selector(showWindow), keyEquivalent: "")
        openItem.target = self
        menu.addItem(openItem)

        menu.addItem(NSMenuItem.separator())

        let pauseItem = NSMenuItem(title: "暂停全部动态壁纸", action: #selector(pauseAll), keyEquivalent: "")
        pauseItem.target = self
        menu.addItem(pauseItem)

        let resumeItem = NSMenuItem(title: "恢复全部", action: #selector(resumeAll), keyEquivalent: "")
        resumeItem.target = self
        menu.addItem(resumeItem)

        let stopItem = NSMenuItem(title: "停止全部动态壁纸", action: #selector(stopAll), keyEquivalent: "")
        stopItem.target = self
        menu.addItem(stopItem)

        menu.addItem(NSMenuItem.separator())

        let quitItem = NSMenuItem(title: "退出", action: #selector(quit), keyEquivalent: "q")
        quitItem.target = self
        menu.addItem(quitItem)

        statusItem?.menu = menu
    }

    @objc func showWindow() {
        for window in NSApp.windows {
            window.makeKeyAndOrderFront(self)
        }
        NSApp.activate(ignoringOtherApps: true)
    }

    @objc func pauseAll() {
        core.pauseAll()
    }

    @objc func resumeAll() {
        core.resumeAll()
    }

    @objc func stopAll() {
        core.stopAll()
    }

    @objc func quit() {
        core.shutdown()
        NSApp.terminate(self)
    }
}
