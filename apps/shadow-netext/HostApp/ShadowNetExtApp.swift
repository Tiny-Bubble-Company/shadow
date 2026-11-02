// Host app for the Shadow transparent proxy: installs the system extension and turns the
// proxy configuration on or off. macOS asks the user to approve the extension once, in
// System Settings > General > Login Items & Extensions > Network Extensions.

import NetworkExtension
import SwiftUI
import SystemExtensions

let extensionID = "dev.shadow.netext.extension"

@main
struct ShadowNetExtApp: App {
    @StateObject private var model = Model()

    var body: some Scene {
        WindowGroup("Shadow capture") {
            VStack(alignment: .leading, spacing: 12) {
                Text("Capture all traffic with Shadow").font(.headline)
                Text("Every app's connections go through the local Shadow proxy. AI traffic is masked; everything else passes through untouched. If Shadow stops, apps connect directly.")
                    .font(.callout).foregroundStyle(.secondary)
                Text(model.status).font(.callout.monospaced())
                HStack {
                    Button("Install extension") { model.install() }
                    Button("Turn on") { model.enable(true) }
                    Button("Turn off") { model.enable(false) }
                }
            }
            .padding(20)
            .frame(width: 460)
        }
    }
}

final class Model: NSObject, ObservableObject, OSSystemExtensionRequestDelegate {
    @Published var status = "Not installed"

    func install() {
        let request = OSSystemExtensionRequest.activationRequest(forExtensionWithIdentifier: extensionID, queue: .main)
        request.delegate = self
        OSSystemExtensionManager.shared.submitRequest(request)
        status = "Waiting for approval in System Settings…"
    }

    func enable(_ on: Bool) {
        NETransparentProxyManager.loadAllFromPreferences { managers, error in
            let manager = managers?.first ?? NETransparentProxyManager()
            let proto = NETunnelProviderProtocol()
            proto.providerBundleIdentifier = extensionID
            proto.serverAddress = "127.0.0.1:8877"
            proto.providerConfiguration = ["shadowHost": "127.0.0.1", "shadowPort": 8877]
            manager.protocolConfiguration = proto
            manager.localizedDescription = "Shadow"
            manager.isEnabled = on
            manager.saveToPreferences { error in
                if let error {
                    DispatchQueue.main.async { self.status = "Error: \(error.localizedDescription)" }
                    return
                }
                manager.loadFromPreferences { _ in
                    do {
                        if on { try manager.connection.startVPNTunnel() } else { manager.connection.stopVPNTunnel() }
                        DispatchQueue.main.async { self.status = on ? "On: all traffic goes through Shadow" : "Off" }
                    } catch {
                        DispatchQueue.main.async { self.status = "Error: \(error.localizedDescription)" }
                    }
                }
            }
        }
    }

    // MARK: OSSystemExtensionRequestDelegate

    func request(_ request: OSSystemExtensionRequest,
                 actionForReplacingExtension existing: OSSystemExtensionProperties,
                 withExtension ext: OSSystemExtensionProperties) -> OSSystemExtensionRequest.ReplacementAction {
        .replace
    }

    func requestNeedsUserApproval(_ request: OSSystemExtensionRequest) {
        status = "Approve Shadow in System Settings > General > Login Items & Extensions"
    }

    func request(_ request: OSSystemExtensionRequest, didFinishWithResult result: OSSystemExtensionRequest.Result) {
        status = result == .completed ? "Installed. Press Turn on." : "Installed after restart"
    }

    func request(_ request: OSSystemExtensionRequest, didFailWithError error: Error) {
        status = "Install failed: \(error.localizedDescription)"
    }
}
