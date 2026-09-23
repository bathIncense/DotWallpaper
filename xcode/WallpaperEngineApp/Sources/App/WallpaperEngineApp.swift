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
            EmptyView()
        }
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
