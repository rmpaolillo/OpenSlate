import SwiftUI

/// OpenSlate per iPhone e iPad: lo stesso cuore Rust dell'app Android
/// (`mobile/core`), che usa gli stessi file della desktop.
@main
struct OpenSlateApp: App {
    @StateObject private var app = AppModel()
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(app)
                // come la desktop: nulla si perde, uscendo la lavagna va nell'archivio
                .onChange(of: phase) { p in
                    if p != .active { app.saveBoardIfNeeded(); app.sync.pause() } else { app.sync.resume() }
                }
                .onAppear { app.sync.resume() }
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
