/*!
# cuda-a2a

Agent-to-Agent protocol implementation.

A2A is how agents communicate. Every message carries:
- Intent (what the agent wants)
- Confidence (how sure they are)  
- Trust (how much they trust the recipient)
- Priority (urgency)
- Payload (the actual data, JSON-first)

Trust decays exponentially, grows on cooperation. Confidence propagates
via Bayesian fusion when agents share observations.

This is the nervous system of the fleet.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A2A message — the fundamental communication unit
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct A2AMessage {
    pub id: u64,
    pub sender: String,
    pub recipient: String,
    pub intent: Intent,
    pub payload: serde_json::Value,
    pub confidence: f64,
    pub trust: f64,
    pub priority: Priority,
    pub timestamp: u64,
    pub in_reply_to: Option<u64>,
    pub ttl: u32, // time-to-live in ticks
}

/// What an agent wants to accomplish
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    // Information
    Observe,      // sharing an observation
    Explain,      // explaining reasoning
    Ask,          // requesting information
    Report,       // status report
    // Coordination
    Propose,      // proposing an action
    Accept,       // accepting a proposal
    Reject,       // rejecting a proposal
    Delegate,     // delegating a task
    Complete,     // task completion report
    // Social
    Greet,        // fleet introduction
    Bond,         // trust building
    Warn,         // alert about danger
    Teach,        // sharing knowledge
    // Resource
    Request,      // requesting energy/data
    Share,        // sharing energy/data
    Release,      // releasing resources
}

impl Intent {
    /// How cooperative is this intent? [0,1]
    pub fn cooperativity(self) -> f64 {
        match self {
            Intent::Share | Intent::Teach | Intent::Accept | Intent::Complete => 1.0,
            Intent::Observe | Intent::Explain | Intent::Report | Intent::Greet
            | Intent::Bond => 0.8,
            Intent::Ask | Intent::Propose | Intent::Delegate => 0.6,
            Intent::Reject | Intent::Warn => 0.3,
            Intent::Request => 0.4,
            Intent::Release => 0.7,
        }
    }
}

/// Message priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Trust score between two agents
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustScore {
    pub agent: String,
    pub score: f64,        // [0,1]
    pub interactions: u32,
    pub cooperations: u32, // successful
    pub defections: u32,   // failed/deceptive
    pub last_seen: u64,
    pub decay_rate: f64,   // per tick
}

impl TrustScore {
    pub fn new(agent: &str) -> Self {
        TrustScore { agent: agent.to_string(), score: 0.5, interactions: 0, cooperations: 0, defections: 0, last_seen: 0, decay_rate: 0.001 }
    }

    /// Record an interaction outcome
    pub fn record(&mut self, cooperative: bool) {
        self.interactions += 1;
        self.last_seen = now();
        if cooperative {
            self.cooperations += 1;
            // Trust grows slowly
            self.score = (self.score + 0.02).min(1.0);
        } else {
            self.defections += 1;
            // Trust drops fast
            self.score = (self.score - 0.1).max(0.0);
        }
    }

    /// Decay trust over time
    pub fn tick(&mut self) {
        self.score = (self.score - self.decay_rate).max(0.0);
    }

    /// Cooperation rate
    pub fn cooperation_rate(&self) -> f64 {
        if self.interactions == 0 { return 0.5; }
        self.cooperations as f64 / self.interactions as f64
    }
}

/// A2A inbox — receives and routes messages
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inbox {
    pub agent_id: String,
    pub messages: Vec<A2AMessage>,
    pub max_size: usize,
    pub sent: Vec<A2AMessage>,
}

impl Inbox {
    pub fn new(agent_id: &str) -> Self {
        Inbox { agent_id: agent_id.to_string(), messages: vec![], max_size: 1000, sent: vec![] }
    }

    /// Receive a message (sorted by priority)
    pub fn receive(&mut self, mut msg: A2AMessage) {
        if self.messages.len() >= self.max_size {
            // Drop lowest priority
            self.messages.sort_by(|a, b| b.priority.cmp(&a.priority));
            self.messages.pop();
        }
        msg.ttl = msg.ttl.saturating_sub(1);
        if msg.ttl > 0 {
            self.messages.push(msg);
            self.messages.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
    }

    /// Send a message
    pub fn send(&mut self, recipient: &str, intent: Intent, payload: serde_json::Value, confidence: f64, trust: f64) -> A2AMessage {
        let msg = A2AMessage {
            id: now(),
            sender: self.agent_id.clone(),
            recipient: recipient.to_string(),
            intent,
            payload,
            confidence: confidence.clamp(0.0, 1.0),
            trust: trust.clamp(0.0, 1.0),
            priority: Priority::Normal,
            timestamp: now(),
            in_reply_to: None,
            ttl: 100,
        };
        self.sent.push(msg.clone());
        msg
    }

    /// Send critical message
    pub fn send_critical(&mut self, recipient: &str, intent: Intent, payload: serde_json::Value) -> A2AMessage {
        let mut msg = self.send(recipient, intent, payload, 1.0, 1.0);
        msg.priority = Priority::Critical;
        msg
    }

    /// Get unread messages for a specific intent
    pub fn by_intent(&self, intent: Intent) -> Vec<&A2AMessage> {
        self.messages.iter().filter(|m| m.intent == intent).collect()
    }

    /// Pop highest priority message
    pub fn next(&mut self) -> Option<A2AMessage> {
        if self.messages.is_empty() { return None; }
        self.messages.remove(0)
    }

    /// Reply to a message
    pub fn reply(&mut self, original: &A2AMessage, intent: Intent, payload: serde_json::Value, confidence: f64) -> A2AMessage {
        let mut msg = self.send(&original.sender, intent, payload, confidence, original.trust);
        msg.in_reply_to = Some(original.id);
        msg
    }
}

/// Fleet-wide A2A router
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FleetRouter {
    pub agents: HashMap<String, Inbox>,
    pub trust_map: HashMap<String, TrustScore>, // "agentA->agentB"
    pub total_messages: u64,
}

impl FleetRouter {
    pub fn new() -> Self { FleetRouter { agents: HashMap::new(), trust_map: HashMap::new(), total_messages: 0 } }

    pub fn add_agent(&mut self, id: &str) { self.agents.insert(id.to_string(), Inbox::new(id)); }

    /// Route a message from sender to recipient
    pub fn route(&mut self, msg: A2AMessage) -> bool {
        if let Some(inbox) = self.agents.get_mut(&msg.recipient) {
            inbox.receive(msg.clone());
            self.total_messages += 1;
            // Update trust
            let trust_key = format!("{}->{}", msg.recipient, msg.sender);
            if let Some(ts) = self.trust_map.get_mut(&trust_key) {
                ts.record(msg.intent.cooperativity() > 0.5);
            }
            true
        } else { false }
    }

    /// Broadcast to all agents except sender
    pub fn broadcast(&mut self, sender: &str, intent: Intent, payload: serde_json::Value, confidence: f64) -> u32 {
        let mut sent = 0u32;
        for (id, _) in &self.agents {
            if id != sender {
                let msg = A2AMessage { id: now() + sent as u64, sender: sender.to_string(), recipient: id.clone(), intent, payload: payload.clone(), confidence, trust: 0.5, priority: Priority::Normal, timestamp: now(), in_reply_to: None, ttl: 50 };
                self.route(msg);
                sent += 1;
            }
        }
        sent
    }

    /// Get or create trust score
    pub fn trust(&self, from: &str, to: &str) -> f64 {
        let key = format!("{}->{}", from, to);
        self.trust_map.get(&key).map(|t| t.score).unwrap_or(0.5)
    }

    /// Decay all trust scores
    pub fn tick(&mut self) {
        for ts in self.trust_map.values_mut() { ts.tick(); }
    }
}

/// Bayesian confidence fusion: combine independent confidence sources
pub fn fuse_confidence(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 { return 0.0; }
    1.0 / (1.0 / a + 1.0 / b)
}

/// Negotiate a proposal between two agents
pub fn negotiate(
    proposer_conf: f64, proposer_trust: f64,
    receiver_conf: f64, receiver_trust: f64,
) -> NegotiationResult {
    let mutu_trust = (proposer_trust + receiver_trust) / 2.0;
    let fused_conf = fuse_confidence(proposer_conf, receiver_conf);
    let threshold = 0.3;

    if fused_conf >= threshold && mutu_trust >= 0.3 {
        NegotiationResult::Accepted { confidence: fused_conf, trust: mutu_trust }
    } else if fused_conf >= threshold * 0.5 {
        NegotiationResult::CounterProposal { suggested_conf: threshold, reason: "insufficient confidence".to_string() }
    } else {
        NegotiationResult::Rejected { reason: format!("confidence {:.3} below threshold {:.3}", fused_conf, threshold) }
    }
}

#[derive(Clone, Debug)]
pub enum NegotiationResult {
    Accepted { confidence: f64, trust: f64 },
    CounterProposal { suggested_conf: f64, reason: String },
    Rejected { reason: String },
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let mut inbox = Inbox::new("agent-1");
        let msg = inbox.send("agent-2", Intent::Ask, serde_json::json!({"q":"status"}), 0.9, 0.7);
        assert_eq!(msg.sender, "agent-1");
        assert_eq!(msg.recipient, "agent-2");
        assert_eq!(msg.intent, Intent::Ask);
        assert!(!inbox.sent.is_empty());
    }

    #[test]
    fn test_receive_route() {
        let mut router = FleetRouter::new();
        router.add_agent("a");
        router.add_agent("b");
        let inbox_a = Inbox::new("a");
        let msg = inbox_a.send("b", Intent::Observe, serde_json::json!({"temp": 42.0}), 0.8, 0.6);
        assert!(router.route(msg));
        assert_eq!(router.total_messages, 1);
        assert!(!router.agents["b"].messages.is_empty());
    }

    #[test]
    fn test_trust_growth() {
        let mut ts = TrustScore::new("agent-2");
        assert_eq!(ts.score, 0.5);
        ts.record(true);
        assert!(ts.score > 0.5);
        ts.record(true);
        assert!(ts.score > 0.52);
    }

    #[test]
    fn test_trust_decay() {
        let mut ts = TrustScore::new("a");
        ts.score = 0.8;
        ts.decay_rate = 0.01;
        for _ in 0..100 { ts.tick(); }
        assert!(ts.score < 0.8);
    }

    #[test]
    fn test_broadcast() {
        let mut router = FleetRouter::new();
        router.add_agent("a");
        router.add_agent("b");
        router.add_agent("c");
        let sent = router.broadcast("a", Intent::Warn, serde_json::json!({"alert":"fire"}), 1.0);
        assert_eq!(sent, 2);
    }

    #[test]
    fn test_reply() {
        let mut inbox = Inbox::new("a");
        let msg = inbox.send("b", Intent::Ask, serde_json::json!({"q":"ping"}), 0.9, 0.7);
        let reply = inbox.reply(&msg, Intent::Report, serde_json::json!({"answer":"pong"}), 0.95);
        assert_eq!(reply.recipient, "a"); // back to self (would normally route)
        assert_eq!(reply.in_reply_to, Some(msg.id));
    }

    #[test]
    fn test_fuse_confidence() {
        let fused = fuse_confidence(0.8, 0.8);
        assert!(fused < 0.8); // always reduces
        assert!(fused > 0.3);
    }

    #[test]
    fn test_fuse_zero() {
        assert_eq!(fuse_confidence(0.0, 0.8), 0.0);
        assert_eq!(fuse_confidence(0.8, 0.0), 0.0);
    }

    #[test]
    fn test_negotiation_accept() {
        let result = negotiate(0.9, 0.8, 0.85, 0.7);
        match result {
            NegotiationResult::Accepted { confidence, trust } => {
                assert!(confidence > 0.3);
                assert!(trust > 0.3);
            }
            _ => panic!("expected accept"),
        }
    }

    #[test]
    fn test_negotiation_reject() {
        let result = negotiate(0.1, 0.1, 0.1, 0.1);
        match result {
            NegotiationResult::Rejected { .. } => {},
            _ => panic!("expected reject"),
        }
    }

    #[test]
    fn test_priority_sorting() {
        let mut inbox = Inbox::new("a");
        inbox.receive(inbox.send("b", Intent::Observe, serde_json::json!(1), 0.5, 0.5));
        let mut crit = inbox.send("b", Intent::Warn, serde_json::json!(2), 1.0, 1.0);
        crit.priority = Priority::Critical;
        inbox.receive(crit);
        let next = inbox.next().unwrap();
        assert_eq!(next.priority, Priority::Critical);
    }

    #[test]
    fn test_ttl_expiration() {
        let mut inbox = Inbox::new("a");
        let mut msg = inbox.send("b", Intent::Observe, serde_json::json!(1), 0.5, 0.5);
        msg.ttl = 1; // will expire on receive
        inbox.receive(msg);
        assert!(inbox.messages.is_empty()); // dropped due to ttl
    }

    #[test]
    fn test_intent_cooperativity() {
        assert!(Intent::Share.cooperativity() > Intent::Request.cooperativity());
        assert!(Intent::Teach.cooperativity() > Intent::Reject.cooperativity());
    }

    #[test]
    fn test_cooperation_rate() {
        let mut ts = TrustScore::new("a");
        ts.record(true); ts.record(true); ts.record(false);
        assert!((ts.cooperation_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_inbox_overflow() {
        let mut inbox = Inbox::new("a");
        inbox.max_size = 3;
        for _ in 0..5 {
            let mut msg = inbox.send("b", Intent::Observe, serde_json::json!(1), 0.5, 0.5);
            msg.ttl = 100;
            inbox.receive(msg);
        }
        assert!(inbox.messages.len() <= 3);
    }
}
