import SwiftUI

/// OpenSlate per iPhone e iPad: lo stesso cuore Rust dell'app Android
/// (`mobile/core`), che usa gli stessi file della desktop.
/// Rotazione automatica dello schermo (Impostazioni): accesa, l'app gira col
/// telefono; spenta, resta in verticale come prima.
final class OrientationDelegate: NSObject, UIApplicationDelegate {
    static var mask: UIInterfaceOrientationMask {
        let auto = UserDefaults.standard.object(forKey: "auto_rotate") as? Bool ?? true
        if !auto { return .portrait }
        return UIDevice.current.userInterfaceIdiom == .pad ? .all : .allButUpsideDown
    }

    func application(_ application: UIApplication, supportedInterfaceOrientationsFor window: UIWindow?) -> UIInterfaceOrientationMask {
        Self.mask
    }

    /// Applica subito la scelta (spegnendola col telefono in orizzontale, torna in verticale).
    static func apply() {
        for case let scene as UIWindowScene in UIApplication.shared.connectedScenes {
            scene.windows.first?.rootViewController?.setNeedsUpdateOfSupportedInterfaceOrientations()
            scene.requestGeometryUpdate(.iOS(interfaceOrientations: mask)) { _ in }
        }
    }
}

@main
struct OpenSlateApp: App {
    @UIApplicationDelegateAdaptor(OrientationDelegate.self) private var orientation
    @StateObject private var app = AppModel()

    init() {
        // i titoli delle schermate nel blu dell'app, come su Android
        let blue = UIColor(named: "AccentColor") ?? .systemBlue
        UINavigationBar.appearance().largeTitleTextAttributes = [.foregroundColor: blue]
        UINavigationBar.appearance().titleTextAttributes = [.foregroundColor: blue]
    }
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(app)
                // come la desktop: nulla si perde, uscendo la lavagna va nell'archivio
                .onChange(of: phase) { p in
                    if p != .active { app.saveBoardIfNeeded(); app.sync.pause(); app.sync.stopLive() } else { app.sync.resume(); app.startLive() }
                }
                .onAppear { app.sync.resume(); app.startLive() }
        }
    }
}

struct RootView: View {
    @EnvironmentObject var app: AppModel

    var body: some View {
        ZStack {
            TabView(selection: Binding(get: { app.tab }, set: { t in
                if t != .board { app.saveBoardIfNeeded() }
                app.tab = t
            })) {
                BoardScreen(board: app.board)
                    .tabItem { Label(L("Lavagna", "Board"), systemImage: "scribble") }.tag(Tab.board)
                LibraryScreen()
                    .tabItem { Label(L("Libreria", "Library"), systemImage: "photo.stack") }.tag(Tab.library)
                ChatScreen(chat: app.chat)
                    .tabItem { Label(L("Chat", "Chat"), systemImage: "bubble.left.and.text.bubble.right") }.tag(Tab.chat)
                SettingsScreen()
                    .tabItem { Label(L("Impostazioni", "Settings"), systemImage: "gearshape") }.tag(Tab.settings)
            }
            if let b = app.busy {
                Color.black.opacity(0.35).ignoresSafeArea()
                VStack(spacing: 12) {
                    ProgressView()
                    Text(b).multilineTextAlignment(.center)
                }
                .padding(24)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16))
            }
            if let t = app.toast {
                VStack {
                    Spacer()
                    Text(t).padding(12).background(.regularMaterial, in: Capsule()).padding(.bottom, 90)
                }
                .transition(.opacity)
            }
        }
    }
}
