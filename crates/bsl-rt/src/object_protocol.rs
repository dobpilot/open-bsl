use std::any::Any;
use std::fmt;
use std::rc::Rc;

use crate::{BslValue, CallContext, RtError, RtResult};

/// Байтовый поток, который можно передать другому runtime-компоненту.
///
/// Протокол не раскрывает конкретный носитель: ZIP, читатели данных и
/// host-объекты работают с одной границей, не зная о `File` или буфере.
pub trait ByteStreamProtocol: fmt::Debug {
    /// Текущая позиция без её изменения.
    fn position(&self, op: &'static str) -> RtResult<u64>;

    /// Переводит позицию, не меняя размер носителя.
    fn set_position(&self, position: u64, op: &'static str) -> RtResult<()>;

    /// Длина носителя в байтах.
    fn len(&self, op: &'static str) -> RtResult<u64>;

    /// Читает не больше `count` байт и сдвигает позицию.
    fn read_bytes(&self, count: usize, op: &'static str) -> RtResult<Vec<u8>>;

    /// Записывает байты с текущей позиции и сдвигает её.
    fn write_bytes(&self, bytes: &[u8], op: &'static str) -> RtResult<()>;

    /// Читает поток целиком и восстанавливает исходную позицию.
    fn read_all(&self, op: &'static str) -> RtResult<Vec<u8>> {
        let len = self.len(op)?;
        let count = usize::try_from(len).map_err(|_| RtError::TypeError {
            expected: "Поток, размер которого умещается в памяти",
            op,
        })?;
        let position = self.position(op)?;
        self.set_position(0, op)?;
        let result = self.read_bytes(count, op);
        let restore = self.set_position(position, op);
        match result {
            Err(error) => Err(error),
            Ok(bytes) => {
                restore?;
                Ok(bytes)
            }
        }
    }

    /// Записывает готовый блок с текущей позиции, не закрывая поток.
    fn write_all(&self, bytes: &[u8], op: &'static str) -> RtResult<()> {
        self.write_bytes(bytes, op)
    }
}

/// Неизменяемое описание типа объекта, принадлежащее компоненту.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeDescriptor {
    pub package: &'static str,
    /// Имя ЗНАЧЕНИЯ и конструктора: «ЧтениеXML».
    pub name: &'static str,
    /// Как тип ПЕЧАТАЕТСЯ — `Строка(ТипЗнч(...))`. У платформы это часто
    /// не имя значения: «Чтение XML» с пробелом, «Документ  DOM» с двумя.
    /// Все написания здесь ИЗМЕРЕНЫ на 8.3.27.
    pub type_display: &'static str,
    /// Дополнительные имена, по которым тип ищет `Тип("Имя")`, сверх
    /// `name` и `type_display`: английское написание и — там, где
    /// представление расходится с именем в коде, — идентификатор
    /// (`ОбъявлениеЭлементаXS` печатается «Объявление элемента XML
    /// Schema»).
    pub type_names: &'static [&'static str],
}

impl TypeDescriptor {
    /// Дескриптор типа, у которого имя значения, представление и имя
    /// поиска — одно и то же слово. Тип с измеренными расхождениями
    /// («ЧтениеXML» против «Чтение XML») заполняет поля литералом.
    pub const fn new(package: &'static str, name: &'static str) -> Self {
        Self {
            package,
            name,
            type_display: name,
            type_names: &[],
        }
    }

    /// Ищется ли тип по этому имени. Регистр не значим, как везде в языке.
    pub fn answers_to(&self, name: &str) -> bool {
        crate::folded_eq(self.name, name)
            || crate::folded_eq(self.type_display, name)
            || self.type_names.iter().any(|n| crate::folded_eq(n, name))
    }
}

/// Служебная основа безопасного downcast и приведения к трейт-объекту без
/// обязательных методов в каждой реализации [`ObjectProtocol`]. `as_dyn`
/// существует потому, что в теле метода по умолчанию `Self: ?Sized` и
/// `self` не приводится к `&dyn ObjectProtocol` напрямую, а `where Self:
/// Sized` выбросил бы метод из vtable — строковый вход VM перестал бы
/// диспетчеризоваться.
#[doc(hidden)]
pub trait ObjectDowncast {
    fn as_any(&self) -> &dyn Any;
    fn as_dyn(&self) -> &dyn ObjectProtocol;
}

impl<T: Any + ObjectProtocol> ObjectDowncast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_dyn(&self) -> &dyn ObjectProtocol {
        self
    }
}

impl dyn ObjectProtocol + '_ {
    /// Конкретная реализация за трейт-объектом, если она имеет тип `T` —
    /// как [`ObjectRef::downcast_ref`], но для получателя, каким его видит
    /// обработчик метода: даункаст без служебного [`ObjectDowncast`] в
    /// области видимости.
    pub fn downcast_ref<T: ObjectProtocol + 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }
}

/// Расширяемый протокол объекта BSL.
///
/// Реализация не получает стек или регистры VM. Операции, которые тип не
/// поддерживает, оставляют реализацию по умолчанию с типизированной
/// ошибкой.
pub trait ObjectProtocol: fmt::Debug + ObjectDowncast {
    fn type_descriptor(&self) -> &'static TypeDescriptor;

    /// Вход для доступа к свойству с именем-строкой. По умолчанию
    /// диспетчеризует по [`ObjectProtocol::property_table`]; пустая
    /// таблица отвечает `UnknownProperty`. Собственная реализация нужна
    /// типу с ДИНАМИЧЕСКИМ набором имён (объекты XDTO, параметры макета) —
    /// такой оставляет таблицу пустой и сворачивает имена сам.
    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<BslValue> {
        crate::get_property_from_table(
            self.property_table(),
            self.type_descriptor().name,
            self.as_dyn(),
            name,
            context,
        )
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        crate::set_property_from_table(
            self.property_table(),
            self.type_descriptor().name,
            self.as_dyn(),
            name,
            value,
            context,
        )
    }

    /// Статическая таблица свойств типа — как
    /// [`ObjectProtocol::method_table`] для методов. Непустая включает
    /// быстрый путь VM «номер имени → обработчик»; пустая оставляет типу
    /// только строковый `get_property`/`set_property`.
    fn property_table(&self) -> &'static [crate::PropertyDescriptor] {
        &[]
    }

    /// Вход для вызовов с именем-строкой — `WriteText`/`CloseText` и
    /// legacy-`CallMethod`. По умолчанию диспетчеризует по
    /// [`ObjectProtocol::method_table`]; пустая таблица даёт прежний
    /// `UnknownMethod` из [`crate::call_method_from_table`]. Собственная
    /// реализация нужна только типу, который обрабатывает вызовы мимо
    /// таблицы (bsl-regexp).
    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        crate::call_method_from_table(
            self.method_table(),
            self.type_descriptor().name,
            self.as_dyn(),
            name,
            arguments,
            context,
        )
    }

    /// Статическая таблица методов типа. Пустая — тип диспетчеризуется
    /// только строковым [`ObjectProtocol::call_method`]. Непустая включает
    /// быстрый путь VM «номер имени → обработчик» (разрешение по строке
    /// один раз на пару «тип, имя», дальше без строковых операций);
    /// строковый вход обслуживает та же таблица через реализацию
    /// `call_method` по умолчанию.
    fn method_table(&self) -> &'static [crate::MethodDescriptor] {
        &[]
    }

    fn get_index(&self, _index: &BslValue) -> RtResult<BslValue> {
        Err(RtError::NotIndexable)
    }

    fn set_index(&self, _index: &BslValue, _value: BslValue) -> RtResult<()> {
        Err(RtError::NotIndexable)
    }

    fn collection_len(&self) -> RtResult<usize> {
        Err(RtError::TypeError {
            expected: "Коллекция",
            op: "Количество",
        })
    }

    fn is_filled(&self) -> RtResult<bool> {
        Err(RtError::TypeError {
            expected: "Значение, у которого есть признак заполненности",
            op: "ЗначениеЗаполнено",
        })
    }

    /// Возвращает потоковую возможность объекта, если он ею является.
    fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        None
    }

    /// Ключ ссылочного равенства для типов, равных «по месту в состоянии»,
    /// а не по тождеству обёртки: адрес состояния и номер внутри него.
    /// Обёртку такие типы строят на каждое обращение к коллекции, поэтому
    /// тождества значений недостаточно. `None` оставляет объекту равенство
    /// только по тождеству.
    fn identity_key(&self) -> Option<(usize, usize)> {
        None
    }

    /// Пары «имя — значение», которые объект отдаёт источником
    /// «ЗаполнитьЗначенияСвойств»: `None` — объект в заполнении не
    /// участвует. Порядок пар — порядок обхода источника.
    fn fill_source_pairs(&self) -> Option<Vec<(String, BslValue)>> {
        None
    }

    /// Есть ли у объекта свойство с таким именем — для приёмника
    /// заполнения, который пропускает чужие имена без ошибки.
    fn has_property(&self, _name: &str) -> bool {
        false
    }

    /// Запись одного свойства приёмником заполнения: `Ok(false)` — такого
    /// свойства нет, и перенос молча пропускает имя (набор свойств
    /// приёмника не растёт — измерено).
    fn fill_property(&self, _name: &str, _value: BslValue) -> RtResult<bool> {
        Ok(false)
    }

    /// Равенство по значению для внешних типов-значений: два отдельно
    /// построенных объекта равны содержимым, как строки. `None` — тип по
    /// значению не сравнивается, и в силе остаются ключ места и
    /// тождество. Сравнение вызывается только для объектов ОДНОГО
    /// дескриптора, поэтому реализация может смело сужать `other` к
    /// собственному типу.
    fn value_eq(&self, _other: &ObjectRef) -> Option<bool> {
        None
    }

    fn display(&self) -> String {
        self.type_descriptor().name.to_string()
    }
}

/// Ссылочное значение внешнего объекта. Клонирование сохраняет
/// тождественность через `Rc`, как у встроенных ссылочных объектов.
#[derive(Clone)]
pub struct ObjectRef(Rc<dyn ObjectProtocol>);

impl ObjectRef {
    pub fn new(object: impl ObjectProtocol + 'static) -> Self {
        Self(Rc::new(object))
    }

    pub fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.0.type_descriptor()
    }

    /// Возвращает конкретную реализацию объекта, если она имеет тип `T`.
    pub fn downcast_ref<T: ObjectProtocol + 'static>(&self) -> Option<&T> {
        self.0.as_ref().as_any().downcast_ref()
    }

    /// Объект за ссылкой как трейт-объект — для быстрых путей VM, где
    /// обработчику передаётся получатель без обёртки значения.
    pub fn as_dyn(&self) -> &dyn ObjectProtocol {
        &*self.0
    }

    pub fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<BslValue> {
        self.0.get_property(name, context)
    }

    pub fn set_property(
        &self,
        name: &str,
        value: BslValue,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.0.set_property(name, value, context)
    }

    pub fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        self.0.call_method(name, arguments, context)
    }

    pub fn method_table(&self) -> &'static [crate::MethodDescriptor] {
        self.0.method_table()
    }

    pub fn property_table(&self) -> &'static [crate::PropertyDescriptor] {
        self.0.property_table()
    }

    pub fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        self.0.get_index(index)
    }

    pub fn set_index(&self, index: &BslValue, value: BslValue) -> RtResult<()> {
        self.0.set_index(index, value)
    }

    pub fn collection_len(&self) -> RtResult<usize> {
        self.0.collection_len()
    }

    pub fn is_filled(&self) -> RtResult<bool> {
        self.0.is_filled()
    }

    pub fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        self.0.byte_stream()
    }

    pub fn identity_key(&self) -> Option<(usize, usize)> {
        self.0.identity_key()
    }

    pub fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
        self.0.value_eq(other)
    }

    pub fn fill_source_pairs(&self) -> Option<Vec<(String, BslValue)>> {
        self.0.fill_source_pairs()
    }

    pub fn has_property(&self, name: &str) -> bool {
        self.0.has_property(name)
    }

    pub fn fill_property(&self, name: &str, value: BslValue) -> RtResult<bool> {
        self.0.fill_property(name, value)
    }

    pub fn display(&self) -> String {
        self.0.display()
    }
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ObjectRef")
            .field(&self.type_descriptor())
            .finish()
    }
}
