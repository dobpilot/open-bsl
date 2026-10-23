use std::{cell::RefCell, rc::Rc};

use open_bsl::{CallContext, Engine, HostError, RtResult, UserMessageDto, UserMessageSink, Value};

#[derive(Default)]
struct PendingHost {
    reads: RefCell<Vec<bool>>,
}

impl UserMessageSink for PendingHost {
    fn enqueue(&self, _: &UserMessageDto) -> Result<(), HostError> {
        Ok(())
    }

    fn pending_messages(&self, remove: bool, _: &mut CallContext<'_>) -> RtResult<Vec<Value>> {
        self.reads.borrow_mut().push(remove);
        Ok(vec![Value::Str("host message".into())])
    }
}

#[test]
fn pending_messages_use_the_current_host_and_measured_boolean_flag() {
    let engine = Engine::builder().build().unwrap();
    let host = Rc::new(PendingHost::default());
    let mut state = engine.state_builder().message_sink(host.clone()).build();
    for expression in [
        "ПолучитьСообщенияПользователю()",
        "GetUserMessages(False)",
        "GetUserMessages(True)",
    ] {
        assert_eq!(
            state.exec(&format!("Возврат {expression}[0];")).unwrap(),
            Value::Str("host message".into())
        );
    }
    assert_eq!(*host.reads.borrow(), vec![false, false, true]);
    for argument in ["Неопределено", "0", "1", "\"True\""] {
        assert!(
            state
                .exec(&format!("Возврат GetUserMessages({argument});"))
                .is_err()
        );
    }
    assert_eq!(host.reads.borrow().len(), 3);
    assert_eq!(
        engine
            .new_state()
            .exec("Возврат ПолучитьСообщенияПользователю().Количество();")
            .unwrap(),
        Value::number_from_i64(0)
    );
}

struct ImmediateHost;

impl UserMessageSink for ImmediateHost {
    fn enqueue(&self, _: &UserMessageDto) -> Result<(), HostError> {
        Ok(())
    }
}

#[test]
fn an_existing_message_sink_keeps_the_empty_pending_default() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine
        .state_builder()
        .message_sink(Rc::new(ImmediateHost))
        .build();
    assert_eq!(
        state
            .exec("Возврат GetUserMessages(True).Количество();")
            .unwrap(),
        Value::number_from_i64(0)
    );
}

#[test]
fn the_pending_host_replays_the_complete_native_message_identity_control() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine
        .state_builder()
        .message_sink(Rc::new(open_bsl::PendingUserMessages::default()))
        .build();
    let source = include_str!(
        "../../../../docs/research/fixtures/session-pending-user-messages/measure.bsl"
    )
    .replace("Сообщить(", "СтрокиПробы.Добавить(");
    let result = state
        .exec(&format!(
            "СтрокиПробы = Новый Массив;\n{source}\nВозврат СтрСоединить(СтрокиПробы, Символы.ПС) + Символы.ПС;"
        ))
        .unwrap();
    assert_eq!(
        result,
        Value::Str(
            include_str!(
                "../../../../docs/research/fixtures/session-pending-user-messages/measure.platform.txt"
            )
            .into()
        )
    );
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct EchoHost(open_bsl::PendingUserMessages);

impl UserMessageSink for EchoHost {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        self.0.enqueue(message)
    }

    fn also_write_to_stdout(&self) -> bool {
        true
    }

    fn pending_messages(
        &self,
        remove: bool,
        context: &mut CallContext<'_>,
    ) -> RtResult<Vec<Value>> {
        self.0.pending_messages(remove, context)
    }
}

#[test]
fn diagnostic_echo_preserves_stdout_without_consuming_pending_messages() {
    let engine = Engine::builder().build().unwrap();
    let output = Capture::default();
    let mut state = engine
        .state_builder()
        .stdout(output.clone())
        .message_sink(Rc::new(EchoHost::default()))
        .build();
    let result = state
        .exec(
            r#"
            Сообщить("global");
            Объект = Новый СообщениеПользователю;
            Объект.Текст = "object";
            Объект.Message();
            Возврат GetUserMessages().Количество();
            "#,
        )
        .unwrap();
    assert_eq!(result, Value::number_from_i64(2));
    assert_eq!(&*output.0.borrow(), b"global\nobject\n");
}
