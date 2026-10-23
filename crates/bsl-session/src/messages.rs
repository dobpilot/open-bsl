//! Очередь сообщений одного сеанса и диагностический вывод хоста.

use open_bsl::{
    CallContext, HostError, PendingUserMessages, RtResult, UserMessageDto, UserMessageSink, Value,
};

#[derive(Default)]
pub(super) struct SessionMessages(PendingUserMessages);

impl UserMessageSink for SessionMessages {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        self.0.enqueue(message)
    }

    fn also_write_to_stdout(&self) -> bool {
        // Консольная диагностика не подтверждает доставку в интерфейс:
        // ожидающие сообщения удаляет только явное чтение с удалением.
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
