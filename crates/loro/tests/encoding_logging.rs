use std::sync::{Arc, Mutex};

use loro::{ExportMode, LoroDoc};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{filter::LevelFilter, layer::Context, prelude::*, Layer};

#[derive(Clone, Default)]
struct EncodingEvents(Arc<Mutex<Vec<Level>>>);

impl<S: Subscriber> Layer<S> for EncodingEvents {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        if metadata.target() == "loro_internal::oplog::change_store::block_encode" {
            self.0.lock().unwrap().push(*metadata.level());
        }
    }
}

fn export_snapshot(level: LevelFilter) -> Vec<Level> {
    let events = EncodingEvents::default();
    let subscriber = tracing_subscriber::registry()
        .with(level)
        .with(events.clone());
    let (snapshot, expected) = tracing::subscriber::with_default(subscriber, || {
        let document = LoroDoc::new();
        document.get_map("root").insert("key", "value").unwrap();
        document.commit();
        (
            document.export(ExportMode::Snapshot).unwrap(),
            document.get_deep_value(),
        )
    });

    let restored = LoroDoc::new();
    restored.import(&snapshot).unwrap();
    assert_eq!(restored.get_deep_value(), expected);

    let captured = events.0.lock().unwrap().clone();
    captured
}

#[test]
fn snapshot_encoding_diagnostics_are_absent_at_info() {
    assert!(export_snapshot(LevelFilter::INFO).is_empty());
}

#[test]
fn snapshot_encoding_diagnostics_have_trace_metadata() {
    // One heading and eight encoded-section sizes describe a single block.
    assert_eq!(export_snapshot(LevelFilter::TRACE), vec![Level::TRACE; 9]);
}
