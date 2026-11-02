// Shadow transparent proxy (macOS Network Extension).
//
// Catches every outbound TCP connection on the Mac, from every app, including apps that
// ignore the system proxy, and hands it to the local Shadow proxy as an HTTP CONNECT
// tunnel. Shadow then does what it already does for proxied traffic: decrypt and mask AI
// hosts, and pass everything else through byte for byte without recording it.
//
// Fail-open: if Shadow is not answering, the flow is connected directly instead.
// QUIC (UDP 443) is refused so browsers fall back to TCP, which Shadow can see.

import Foundation
import Network
import NetworkExtension
import os.log

final class TransparentProxyProvider: NETransparentProxyProvider {
    private let log = Logger(subsystem: "dev.shadow.netext", category: "proxy")
    private let queue = DispatchQueue(label: "dev.shadow.netext.flows")

    /// Where Shadow listens. Set by the host app in the configuration's providerConfiguration.
    private var shadowHost = "127.0.0.1"
    private var shadowPort: UInt16 = 8877

    /// Shadow's own connections are never redirected (that would loop). Matches a Developer
    /// ID build (`dev.shadow.*`) and an ad-hoc signed `cargo` build (`shadow-<hash>`).
    private func isShadow(_ id: String) -> Bool {
        id == "shadow" || id.hasPrefix("shadow-") || id.hasPrefix("dev.shadow.")
    }

    override func startProxy(options: [String: Any]? = nil, completionHandler: @escaping (Error?) -> Void) {
        if let cfg = (protocolConfiguration as? NETunnelProviderProtocol)?.providerConfiguration {
            if let h = cfg["shadowHost"] as? String { shadowHost = h }
            if let p = cfg["shadowPort"] as? Int, let port = UInt16(exactly: p) { shadowPort = port }
        }
        let settings = NETransparentProxyNetworkSettings(tunnelRemoteAddress: shadowHost)
        settings.includedNetworkRules = [
            // All outbound TCP, and UDP 443 so QUIC can be refused.
            NENetworkRule(remoteNetwork: nil, remotePrefix: 0, localNetwork: nil, localPrefix: 0,
                          protocol: .TCP, direction: .outbound),
            NENetworkRule(remoteNetwork: NWHostEndpoint(hostname: "0.0.0.0", port: "443"), remotePrefix: 0,
                          localNetwork: nil, localPrefix: 0, protocol: .UDP, direction: .outbound),
        ]
        settings.excludedNetworkRules = ["127.0.0.0", "10.0.0.0", "172.16.0.0", "192.168.0.0", "169.254.0.0"]
            .enumerated()
            .map { i, net in
                NENetworkRule(destinationNetwork: NWHostEndpoint(hostname: net, port: "0"),
                              prefix: [8, 8, 12, 16, 16][i], protocol: .any)
            }
        setTunnelNetworkSettings(settings) { error in
            if let error { self.log.error("settings failed: \(error.localizedDescription, privacy: .public)") }
            completionHandler(error)
        }
    }

    override func stopProxy(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        completionHandler()
    }

    /// Return `true` to take the flow, `false` to let it go out directly.
    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
        if isShadow(flow.metaData.sourceAppSigningIdentifier) {
            return false
        }
        if flow is NEAppProxyUDPFlow {
            // QUIC: refuse it so the app retries over TCP.
            flow.closeReadWithError(nil)
            flow.closeWriteWithError(nil)
            return true
        }
        guard let tcp = flow as? NEAppProxyTCPFlow,
              let endpoint = tcp.remoteEndpoint as? NWHostEndpoint else {
            return false
        }
        let host = tcp.remoteHostname ?? endpoint.hostname
        let port = endpoint.port
        tunnel(tcp, to: host, port: port)
        return true
    }

    // MARK: - Tunnelling

    private func tunnel(_ flow: NEAppProxyTCPFlow, to host: String, port: String) {
        let shadow = NWConnection(host: NWEndpoint.Host(shadowHost),
                                  port: NWEndpoint.Port(rawValue: shadowPort)!, using: .tcp)
        shadow.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            switch state {
            case .ready:
                self.connect(flow, via: shadow, host: host, port: port)
            case .failed, .waiting:
                // Shadow is not running: go direct rather than cut the app off.
                shadow.cancel()
                self.direct(flow, host: host, port: port)
            default:
                break
            }
        }
        shadow.start(queue: queue)
    }

    private func connect(_ flow: NEAppProxyTCPFlow, via shadow: NWConnection, host: String, port: String) {
        let request = "CONNECT \(host):\(port) HTTP/1.1\r\nHost: \(host):\(port)\r\n\r\n"
        shadow.send(content: request.data(using: .utf8), completion: .contentProcessed { _ in })
        readHeader(shadow, buffer: Data()) { [weak self] ok, rest in
            guard let self, ok else {
                shadow.cancel()
                self?.direct(flow, host: host, port: port)
                return
            }
            flow.open(withLocalEndpoint: nil) { error in
                if error != nil { shadow.cancel(); return }
                if !rest.isEmpty { flow.write(rest) { _ in } }
                self.pipe(flow, shadow)
            }
        }
    }

    /// Read Shadow's reply to CONNECT up to the blank line; `ok` when it is a 200.
    private func readHeader(_ c: NWConnection, buffer: Data, done: @escaping (Bool, Data) -> Void) {
        c.receive(minimumIncompleteLength: 1, maximumLength: 4096) { data, _, _, error in
            guard let data, error == nil else { return done(false, Data()) }
            let all = buffer + data
            if let end = all.range(of: Data("\r\n\r\n".utf8)) {
                let head = String(decoding: all[..<end.lowerBound], as: UTF8.self)
                done(head.hasPrefix("HTTP/1.1 200") || head.hasPrefix("HTTP/1.0 200"), all[end.upperBound...])
            } else if all.count > 16_384 {
                done(false, Data())
            } else {
                self.readHeader(c, buffer: all, done: done)
            }
        }
    }

    /// Connect the flow straight to its destination (Shadow unavailable).
    private func direct(_ flow: NEAppProxyTCPFlow, host: String, port: String) {
        guard let p = NWEndpoint.Port(port) else { return }
        let c = NWConnection(host: NWEndpoint.Host(host), port: p, using: .tcp)
        c.stateUpdateHandler = { [weak self] state in
            if case .ready = state {
                flow.open(withLocalEndpoint: nil) { error in
                    if error == nil { self?.pipe(flow, c) } else { c.cancel() }
                }
            }
        }
        c.start(queue: queue)
    }

    /// Copy bytes both ways until either side closes.
    private func pipe(_ flow: NEAppProxyTCPFlow, _ c: NWConnection) {
        func up() {
            flow.readData { data, error in
                guard let data, !data.isEmpty, error == nil else {
                    c.send(content: nil, contentContext: .finalMessage, isComplete: true, completion: .idempotent)
                    return
                }
                c.send(content: data, completion: .contentProcessed { e in if e == nil { up() } })
            }
        }
        func down() {
            c.receive(minimumIncompleteLength: 1, maximumLength: 65_536) { data, _, done, error in
                if let data, !data.isEmpty {
                    flow.write(data) { e in if e == nil && !done { down() } }
                }
                if done || error != nil {
                    flow.closeReadWithError(nil)
                    flow.closeWriteWithError(nil)
                    c.cancel()
                }
            }
        }
        up()
        down()
    }
}
