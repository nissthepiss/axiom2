use crate::events::trade_event::TelemetryEvent;
use tokio::sync::broadcast;
use tracing::debug;

/// Async event bus for TelemetryEvents.
/// Multiple consumers can subscribe independently.
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<TelemetryEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: TelemetryEvent) {
        match self.sender.send(event) {
            Ok(receiver_count) => {
                debug!("TelemetryEvent published to {} receivers", receiver_count);
            }
            Err(_) => {
                debug!("No active receivers for TelemetryEvent");
            }
        }
    }

    /// Subscribe to telemetry events
    pub fn subscribe(&self) -> broadcast::Receiver<TelemetryEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000)
    }
}
