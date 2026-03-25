use crate::events::trade_event::TradeEvent;
use tokio::sync::broadcast;
use tracing::debug;

/// Internal async event bus for TradeEvents
/// Multiple consumers can subscribe to receive events
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<TradeEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish a trade event to all subscribers
    pub fn publish(&self, event: TradeEvent) {
        match self.sender.send(event) {
            Ok(receiver_count) => {
                debug!("TradeEvent published to {} receivers", receiver_count);
            }
            Err(_) => {
                debug!("No active receivers for TradeEvent");
            }
        }
    }

    /// Subscribe to trade events
    pub fn subscribe(&self) -> broadcast::Receiver<TradeEvent> {
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
