//! Параметры сеанса: состояние слотов, ленивая установка и глобальное
//! свойство `ПараметрыСеанса`.

use super::*;

#[derive(Debug, Clone)]
enum ParameterState {
    Unset,
    Initializing,
    Set(Value),
}

#[derive(Debug, Clone)]
pub(crate) struct ParameterSlot {
    pub(crate) name: String,
    state: ParameterState,
}

/// Контейнер параметров одного сеанса.
#[derive(Debug)]
pub struct SessionParameters {
    handler_module: String,
    pub(crate) slots: RefCell<Vec<ParameterSlot>>,
    positions: Arc<std::collections::HashMap<String, usize>>,
}

impl SessionParameters {
    /// Создаёт контейнер из полного списка параметров конфигурации.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого имени модуля, пустого имени параметра
    /// или двух регистронезависимо одинаковых имён.
    pub fn new(
        handler_module: impl Into<String>,
        names: impl IntoIterator<Item = String>,
    ) -> Result<Self, ParameterError> {
        let handler_module = handler_module.into();
        if handler_module.is_empty() {
            return Err(ParameterError::EmptyHandlerModule);
        }
        let mut slots = Vec::<ParameterSlot>::new();
        let mut positions = std::collections::HashMap::new();
        for name in names {
            if name.is_empty() {
                return Err(ParameterError::EmptyParameterName);
            }
            if positions.insert(name.to_uppercase(), slots.len()).is_some() {
                return Err(ParameterError::DuplicateParameterName(name));
            }
            slots.push(ParameterSlot {
                name,
                state: ParameterState::Unset,
            });
        }
        Ok(Self {
            handler_module,
            slots: RefCell::new(slots),
            positions: Arc::new(positions),
        })
    }

    /// Возвращает имя модуля обработчика конфигурации.
    #[must_use]
    pub fn handler_module(&self) -> &str {
        &self.handler_module
    }

    /// Устанавливает значение параметра без запуска обработчика.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для имени, которого нет в конфигурации.
    pub fn set(&self, name: &str, value: Value) -> Result<(), ParameterError> {
        let mut slots = self.slots.borrow_mut();
        let slot = self
            .find_slot_mut(&mut slots, name)
            .ok_or_else(|| ParameterError::UnknownParameter(name.into()))?;
        slot.state = ParameterState::Set(value);
        Ok(())
    }

    /// Возвращает уже установленное значение без ленивой инициализации.
    /// `None` означает известный, но ещё не установленный параметр.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для имени, которого нет в конфигурации.
    pub fn peek(&self, name: &str) -> Result<Option<Value>, ParameterError> {
        let slots = self.slots.borrow();
        let slot = self
            .find_slot(&slots, name)
            .ok_or_else(|| ParameterError::UnknownParameter(name.into()))?;
        Ok(match &slot.state {
            ParameterState::Set(value) => Some(value.clone()),
            ParameterState::Unset | ParameterState::Initializing => None,
        })
    }

    /// Сбрасывает все значения в состояние `Unset`.
    pub fn clear(&self) {
        for slot in self.slots.borrow_mut().iter_mut() {
            slot.state = ParameterState::Unset;
        }
    }

    /// Создаёт независимую копию для фонового сеанса.
    /// Установленные и неустановленные значения копируются, а незавершённая
    /// синхронная инициализация превращается в `Unset`.
    #[must_use]
    pub fn copy_for_background(&self) -> Self {
        let slots = self
            .slots
            .borrow()
            .iter()
            .map(|slot| ParameterSlot {
                name: slot.name.clone(),
                state: match &slot.state {
                    ParameterState::Set(value) => ParameterState::Set(value.clone()),
                    ParameterState::Unset | ParameterState::Initializing => ParameterState::Unset,
                },
            })
            .collect();
        Self {
            handler_module: self.handler_module.clone(),
            slots: RefCell::new(slots),
            positions: Arc::clone(&self.positions),
        }
    }

    /// Вызывает стартовую или выборочную инициализацию параметров.
    /// `None` передаёт обработчику `Неопределено`, `Some` — массив имён.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку вызова обработчика. При выборочном вызове также
    /// проверяет существование каждого имени и то, что обработчик установил
    /// запрошенные значения.
    pub fn initialize(
        &self,
        context: &mut CallContext<'_>,
        required: Option<&[String]>,
    ) -> RtResult<()> {
        if let Some(required) = required {
            {
                let mut slots = self.slots.borrow_mut();
                for name in required {
                    let slot = self
                        .find_slot_mut(&mut slots, name)
                        .ok_or_else(|| parameter_access_error(name))?;
                    match slot.state {
                        ParameterState::Unset => slot.state = ParameterState::Initializing,
                        ParameterState::Initializing => return Err(parameter_access_error(name)),
                        ParameterState::Set(_) => {}
                    }
                }
            }
        }

        let argument = required.map_or(Value::Undefined, |required| {
            Value::new_array(
                required
                    .iter()
                    .map(|name| Value::Str(name.as_str().into()))
                    .collect(),
            )
        });
        let result = context.call_configuration_handler(
            &self.handler_module,
            "УстановкаПараметровСеанса",
            vec![argument],
            open_bsl::ConfigurationHandlerEntryMode::Privileged,
        );

        if let Some(required) = required {
            let mut slots = self.slots.borrow_mut();
            if result.is_err() {
                for name in required {
                    if let Some(slot) = self.find_slot_mut(&mut slots, name)
                        && matches!(slot.state, ParameterState::Initializing)
                    {
                        slot.state = ParameterState::Unset;
                    }
                }
            } else {
                for name in required {
                    let slot = self
                        .find_slot_mut(&mut slots, name)
                        .ok_or_else(|| parameter_access_error(name))?;
                    if matches!(slot.state, ParameterState::Initializing) {
                        slot.state = ParameterState::Unset;
                        return Err(parameter_access_error(name));
                    }
                }
            }
        }
        result.map(|_| ())
    }

    /// Читает параметр, при необходимости вызывая его выборочную инициализацию.
    ///
    /// Адаптер данных использует тот же путь, что и обращение из BSL: цикл,
    /// неизвестное имя и возврат обработчика без значения остаются ошибками.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для неизвестного имени, повторного чтения во время
    /// инициализации или ошибки обработчика модуля сеанса.
    pub fn get(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        {
            let slots = self.slots.borrow();
            let slot = self
                .find_slot(&slots, name)
                .ok_or_else(|| parameter_access_error(name))?;
            match &slot.state {
                ParameterState::Set(value) => return Ok(value.clone()),
                ParameterState::Initializing => return Err(parameter_access_error(name)),
                ParameterState::Unset => {}
            }
        }
        self.initialize(context, Some(&[name.to_string()]))?;
        self.peek(name)
            .map_err(|_| parameter_access_error(name))?
            .ok_or_else(|| parameter_access_error(name))
    }

    // Индекс неизменяем: копия фонового сеанса разделяет только имена и
    // позиции, а значения и состояния инициализации остаются независимыми.
    fn find_slot<'a>(&self, slots: &'a [ParameterSlot], name: &str) -> Option<&'a ParameterSlot> {
        self.positions
            .get(&name.to_uppercase())
            .and_then(|&index| slots.get(index))
    }

    fn find_slot_mut<'a>(
        &self,
        slots: &'a mut [ParameterSlot],
        name: &str,
    ) -> Option<&'a mut ParameterSlot> {
        self.positions
            .get(&name.to_uppercase())
            .and_then(|&index| slots.get_mut(index))
    }
}

fn parameter_access_error(name: &str) -> RtError {
    RtError::DynamicError(format!("Error getting value of context attribute ({name})"))
}

/// Ошибка объявления параметров конфигурации.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterError {
    /// Имя модуля обработчика пусто.
    EmptyHandlerModule,
    /// Имя параметра пусто.
    EmptyParameterName,
    /// Имя параметра повторяется без учёта регистра.
    DuplicateParameterName(String),
    /// Параметр отсутствует в конфигурации.
    UnknownParameter(String),
}

impl fmt::Display for ParameterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyHandlerModule => formatter.write_str("имя модуля сеанса пусто"),
            Self::EmptyParameterName => formatter.write_str("имя параметра сеанса пусто"),
            Self::DuplicateParameterName(name) => {
                write!(formatter, "параметр сеанса `{name}` повторяется")
            }
            Self::UnknownParameter(name) => {
                write!(formatter, "параметр сеанса `{name}` не найден")
            }
        }
    }
}

impl Error for ParameterError {}

#[derive(Debug)]
struct ParametersObject {
    parameters: Rc<SessionParameters>,
}

pub(crate) static PARAMETERS_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-session", "ПараметрыСеанса");

impl ObjectProtocol for ParametersObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PARAMETERS_TYPE
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        self.parameters.get(name, context)
    }

    fn set_property(&self, name: &str, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
        self.parameters
            .set(name, value)
            .map_err(|_| parameter_access_error(name))
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        PARAMETERS_METHODS
    }
}

fn clear_parameters(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let parameters = receiver
        .downcast_ref::<ParametersObject>()
        .ok_or(RtError::NotAnObject)?;
    parameters.parameters.clear();
    Ok(Value::Undefined)
}

pub(crate) static PARAMETERS_METHODS: &[MethodDescriptor] =
    &[
        MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), clear_parameters)
            .as_procedure(),
    ];

pub(crate) fn parameters(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(ParametersObject {
        parameters: Rc::clone(runtime(ctx)?.parameters()),
    }))
}
