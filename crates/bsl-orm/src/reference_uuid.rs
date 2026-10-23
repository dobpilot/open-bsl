//! UUID ссылок создаются отдельно от конструктора случайного UUID версии 4.
//! Формат версии 1 задан RFC 9562, раздел 5.1; измеренная платформа сохраняет
//! узел и последовательность часов при последовательной генерации ссылок.

use open_bsl::{RandomHandle, RtError, RtResult};

#[derive(Default)]
pub(super) struct ReferenceUuids {
    last_timestamp: Option<u64>,
    node_and_sequence: Option<[u8; 8]>,
}

impl ReferenceUuids {
    pub(super) fn next(&mut self, unix_millis: i64, random: &RandomHandle) -> RtResult<[u8; 16]> {
        // Смещение между эпохами Unix и UUID в интервалах по 100 нс.
        const EPOCH_OFFSET: i128 = 122_192_928_000_000_000;
        let timestamp = i128::from(unix_millis) * 10_000 + EPOCH_OFFSET;
        let timestamp = u64::try_from(timestamp)
            .ok()
            .filter(|timestamp| *timestamp < (1 << 60))
            .ok_or_else(|| RtError::DynamicError("время вне диапазона UUID версии 1".into()))?;
        // Часы хоста могут стоять или двигаться назад. Сеансовый счётчик
        // сохраняет уникальность без состояния процесса и ожидания новых часов.
        let timestamp = match self.last_timestamp {
            Some(previous) if timestamp <= previous => previous + 1,
            _ => timestamp,
        };
        if timestamp >= (1 << 60) {
            return Err(RtError::DynamicError(
                "исчерпан диапазон UUID версии 1".into(),
            ));
        }
        let node_and_sequence = self.node_and_sequence.get_or_insert_with(|| {
            let mut bytes = [0; 16];
            random.fill(&mut bytes);
            let mut node = [0; 8];
            node.copy_from_slice(&bytes[..8]);
            node[0] = (node[0] & 0x3f) | 0x80;
            // Хост использует случайный узел, не аппаратный MAC-адрес.
            // Младший бит первого байта узла отличает его от IEEE-адреса.
            node[2] |= 1;
            node
        });
        let mut uuid = [0; 16];
        uuid[..4].copy_from_slice(&(timestamp as u32).to_be_bytes());
        uuid[4..6].copy_from_slice(&((timestamp >> 32) as u16).to_be_bytes());
        uuid[6..8].copy_from_slice(&(((timestamp >> 48) as u16) | 0x1000).to_be_bytes());
        uuid[8..].copy_from_slice(node_and_sequence);
        self.last_timestamp = Some(timestamp);
        Ok(uuid)
    }
}

#[cfg(test)]
#[path = "reference_uuid_tests.rs"]
mod tests;
