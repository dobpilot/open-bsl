use super::*;
use std::cell::RefCell;

struct CrossSessionObserver {
    other: RefCell<Session>,
    observed: RefCell<Vec<(bool, bool)>>,
}

impl DataAccess for CrossSessionObserver {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let current = context.data_separation_safe_mode("НаименованиеЯзык1")?;
        // Второй сеанс входит в VM до выхода активного кадра первого.
        // Последовательные вызовы после возврата не обнаружили бы утечку
        // через общее состояние потока, очищаемое при выходе из кадра.
        let mut other_session = self.other.borrow_mut();
        let module = other_session
            .instantiate("Функция Наблюдать() Экспорт Возврат Константы.НаблюдениеРежима.Получить(); КонецФункции")
            .unwrap();
        let value = other_session.call(&module, "Наблюдать", vec![]).unwrap().0;
        let Value::Boolean(other) = value else {
            panic!("наблюдатель второго сеанса должен вернуть Булево");
        };
        self.observed.borrow_mut().push((current, other));
        Ok(Value::Boolean(current))
    }
}

#[test]
fn another_session_cannot_observe_a_mode_while_the_enabling_frame_is_active() {
    let factory = factory();
    let resources = |number| {
        SessionResources::new(Box::new(()), number)
            .with_data_separation_common_attributes(["НаименованиеЯзык1".to_string()])
    };
    let other = factory
        .open(
            Principal::new("other", "", vec![]),
            resources(2).with_data_access(Rc::new(ModeObserver)),
        )
        .unwrap();
    let observer = Rc::new(CrossSessionObserver {
        other: RefCell::new(other),
        observed: RefCell::new(Vec::new()),
    });
    let mut current = factory
        .open(
            Principal::new("current", "", vec![]),
            resources(1).with_data_access(observer.clone()),
        )
        .unwrap();
    current
        .execute("SetDataSeparationSafeMode(\"НаименованиеЯзык1\", Истина); Константы.НаблюдениеРежима.Получить();")
        .unwrap();
    current
        .execute("Константы.НаблюдениеРежима.Получить();")
        .unwrap();
    assert_eq!(*observer.observed.borrow(), [(true, false), (false, false)]);
}
