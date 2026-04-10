# cuda-a2a

**Agent-to-Agent communication protocol for the Lucineer fleet.**

> Agents don't work alone. They negotiate, share, warn, and coordinate.
> A2A is the nervous system of the fleet.

## Message Types (12 Intents)

- `Query` / `Response` - Information exchange
- `Request` / `Offer` / `Accept` / `Reject` - Negotiation
- `Inform` / `Warn` / `Command` - Coordination
- `Apologize` / `Thank` - Social bonding

## Key Components

- **`A2AMessage`** - Structured message with sender, intent, payload, priority, confidence
- **`TrustScore`** - Per-agent trust tracking with exponential decay
- **`Inbox`** - Priority queue with deduplication and TTL
- **`FleetRouter`** - Routes messages based on capabilities and trust
- **`fuse_confidence()`** - Bayesian confidence propagation across agents

## How Confidence Flows Through A2A

1. Agent A sends a message with confidence 0.8
2. Agent B's inbox receives it and checks sender trust (0.7)
3. Effective confidence = harmonic_mean(0.8, 0.7) = 0.369
4. Agent B processes only if effective confidence exceeds threshold
5. Agent B's response carries its own confidence, fused with the request

## Ecosystem Integration

- `cuda-equipment` - FleetMessage base type
- `cuda-trust` - TrustScore integrated with multi-context trust
- `cuda-communication` - Higher-level conversation threading
- `cuda-fleet-mesh` - Network topology for message routing
- `cuda-deliberation` - A2A messages carry proposals
- `cuda-compliance` - Policy rules filter A2A messages
- `cuda-did` - DID-based agent identity verification

## See Also

- [cuda-communication](https://github.com/Lucineer/cuda-communication) - Natural language layer
- [cuda-trust](https://github.com/Lucineer/cuda-trust) - Multi-context trust
- [cuda-fleet-mesh](https://github.com/Lucineer/cuda-fleet-mesh) - Fleet network topology
- [cuda-did](https://github.com/Lucineer/cuda-did) - Decentralized identity

## License

MIT OR Apache-2.0