use std::sync::Arc;

/// Зарегистрированный хостом декодер неизменяемого значения.
///
/// Проверяет полезную нагрузку и создаёт новое значение в принимающем сеансе.
/// Он не является конструктором BSL и не передаётся в графе значений.
pub type ValueMaterializer = dyn Fn(&[u8]) -> crate::RtResult<crate::BslValue> + Send + Sync;

/// Неизменяемое описание типа значения, объявленного хостом.
///
/// В отличие от статического [`crate::TypeDescriptor`], строки принадлежат
/// описанию. Хост создаёт его один раз для конфигурации и передаёт движку;
/// значения удерживают описание через `Arc`, без утечки в статическую память.
#[derive(Clone)]
pub struct ValueTypeDescriptor {
    package: &'static str,
    identity: Arc<str>,
    display: Arc<str>,
    names: Vec<String>,
    materializer: Option<Arc<ValueMaterializer>>,
}

impl ValueTypeDescriptor {
    /// Создаёт описание с отдельными идентичностью, представлением и именами
    /// поиска. Представление само по себе не становится именем поиска.
    ///
    /// Непустые поля и конфликты имён проверяются при сборке движка.
    #[must_use]
    pub fn new(
        package: &'static str,
        identity: impl Into<Arc<str>>,
        display: impl Into<Arc<str>>,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            package,
            identity: identity.into(),
            display: display.into(),
            names: names.into_iter().map(Into::into).collect(),
            materializer: None,
        }
    }

    /// Разрешает восстановление неизменяемого значения зарегистрированным
    /// декодером. Декодер должен проверять размер и содержимое нагрузки;
    /// описанию конфигурационного типа не следует удерживать данные сеанса.
    #[must_use]
    pub fn with_materializer(mut self, materializer: Arc<ValueMaterializer>) -> Self {
        self.materializer = Some(materializer);
        self
    }

    /// Есть ли у типа явно зарегистрированный декодер снимка.
    pub fn has_materializer(&self) -> bool {
        self.materializer.is_some()
    }

    /// Восстанавливает значение и проверяет его объявленную идентичность.
    ///
    /// # Errors
    ///
    /// Ошибка декодера, отсутствие декодера или возврат значения другого типа.
    pub fn materialize(&self, bytes: &[u8]) -> crate::RtResult<crate::BslValue> {
        let decoder = self.materializer.as_ref().ok_or_else(|| {
            crate::RtError::DynamicError("декодер типа значения не зарегистрирован".into())
        })?;
        let value = decoder(bytes)?;
        if let crate::BslValue::Type(crate::TypeRef::Value(ty)) = value.type_of()?
            && ty.package() == self.package
            && ty.identity() == self.identity()
        {
            return Ok(value);
        }
        Err(crate::RtError::DynamicError(
            "декодер вернул значение другого типа".into(),
        ))
    }

    /// Компонент, объявивший тип.
    pub const fn package(&self) -> &'static str {
        self.package
    }

    /// Устойчивый ключ типа внутри компонента.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Представление типа для `Строка(ТипЗнч(...))`.
    pub fn display(&self) -> &str {
        &self.display
    }

    /// Явно объявленные имена поиска, русское написание первым.
    pub fn names(&self) -> &[String] {
        &self.names
    }
}

impl std::fmt::Debug for ValueTypeDescriptor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ValueTypeDescriptor")
            .field("package", &self.package)
            .field("identity", &self.identity)
            .field("display", &self.display)
            .field("names", &self.names)
            .field("has_materializer", &self.has_materializer())
            .finish()
    }
}
