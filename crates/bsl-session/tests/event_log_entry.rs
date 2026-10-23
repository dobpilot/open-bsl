use bsl_session::{EventLogEntry, EventLogLevel, EventLogTransactionMode, EventLogWriter};
use open_bsl::{RtResult, Value};
use std::cell::RefCell;

#[derive(Debug, Default)]
struct Journal(RefCell<Vec<EventLogEntry>>);

impl EventLogWriter for Journal {
    fn write(&self, entry: EventLogEntry) -> RtResult<()> {
        self.0.borrow_mut().push(entry);
        Ok(())
    }
}

#[test]
fn a_host_adapter_constructs_an_entry_and_delivers_its_explicit_values() {
    let journal = Journal::default();
    let metadata = Value::Str("metadata payload".into());
    let data = Value::new_array(vec![Value::number_from_i64(42)]);
    let entry = EventLogEntry::new(
        "host.event",
        EventLogLevel::Information,
        metadata.clone(),
        data.clone(),
        "host comment",
        EventLogTransactionMode::Independent,
    );
    assert!(journal.0.borrow().is_empty());
    journal.write(entry).unwrap();
    let entries = journal.0.borrow();
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.event_name(), "host.event");
    assert_eq!(entry.level(), EventLogLevel::Information);
    assert_eq!(entry.metadata(), &metadata);
    assert_eq!(entry.data(), &data);
    assert_eq!(entry.comment(), "host comment");
    assert_eq!(
        entry.transaction_mode(),
        EventLogTransactionMode::Independent
    );
}
