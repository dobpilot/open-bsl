use super::*;
use crate::functional_options::tests::native;
use bsl_orm::{DataAccess, Principal, QueryRequest, QueryResultData, SessionData};
use open_bsl::{CallContext, RtError, RtResult, Value};
use open_sdbl::metadata::{InformationRegisterPeriodicity, InformationRegisterWriteMode};
use std::rc::Rc;

#[derive(Debug)]
struct RejectRead {
    dimensions: Vec<String>,
    resources: Vec<String>,
    day: bool,
}

impl DataAccess for RejectRead {
    fn constant_value(
        &self,
        _: &bsl_orm::MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        unreachable!("проверяется только прямое чтение регистра")
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "selected-user");
        assert!(request.text().contains("РегистрСведений.ОпцииПробы"));
        let parameters = request.parameters().collect::<Vec<_>>();
        let offset = usize::from(self.day);
        assert_eq!(parameters.len(), self.dimensions.len() + offset);
        if self.day {
            assert_eq!(parameters[0].0, "ПериодЧтения");
            assert!(matches!(parameters[0].1, Value::Date(_)));
            assert!(request.text().contains("Данные.Период = &ПериодЧтения"));
        }
        for (index, dimension) in self.dimensions.iter().enumerate() {
            assert!(request.text().contains(&format!(
                "Данные.{dimension} = &{}",
                parameters[index + offset].0
            )));
            assert_eq!(
                parameters[index + offset].1,
                &Value::number_from_i64(index as i64 + 7)
            );
        }
        let positions = self
            .resources
            .iter()
            .enumerate()
            .map(|(index, name)| {
                request
                    .text()
                    .find(&format!("Данные.{name} КАК РесурсЧтения{index}"))
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        Err(RtError::DynamicError("projection-reader-rejected".into()))
    }
}

fn read_error(change: usize) -> String {
    let (catalog, metadata) = native("register-native", |descriptors| {
        // Типы задаются явно для проверки адаптации SDK → ORM, а не как новый
        // эталон платформы. Исходные файлы нативного контроля не меняются.
        for field in descriptors.iter_mut() {
            let digits = match field.field_purpose {
                Some(open_sdbl::metadata::ConfigFieldPurpose::InformationRegisterDimension) => "5",
                Some(open_sdbl::metadata::ConfigFieldPurpose::InformationRegisterResource) => "15",
                _ => continue,
            };
            field.declared_types = Some(vec![ConfigDeclaredType {
                tag: "N".into(),
                reference: None,
                qualifiers: [digits, "0", "0"]
                    .into_iter()
                    .map(|s| open_sdbl::metadata::Value::Atom(s.into()))
                    .collect(),
            }]);
        }
        let owner = descriptors
            .iter_mut()
            .find(|d| d.name == "ОпцииПробы")
            .unwrap();
        match change {
            0 => {}
            1 => owner.information_register_periodicity = None,
            2 => owner.information_register_dimensions = None,
            3 => owner.information_register_resources = None,
            4 => {
                owner.information_register_write_mode =
                    Some(InformationRegisterWriteMode::RecorderSubordinate)
            }
            5 => owner.information_register_write_mode = None,
            6 => owner.information_register_periodicity = Some(InformationRegisterPeriodicity::Day),
            7 => owner.information_register_dimensions = Some(vec![]),
            8 => owner.information_register_periodicity = Some(InformationRegisterPeriodicity::Day),
            9 => {
                owner
                    .information_register_dimensions
                    .as_mut()
                    .unwrap()
                    .reverse();
                owner
                    .information_register_resources
                    .as_mut()
                    .unwrap()
                    .reverse();
            }
            _ => unreachable!(),
        }
    });
    let owner = metadata
        .descriptors()
        .iter()
        .find(|d| d.name == "ОпцииПробы")
        .unwrap();
    let names = owner
        .information_register_dimensions
        .as_ref()
        .into_iter()
        .flatten()
        .map(|id| {
            metadata
                .descriptors()
                .iter()
                .find(|d| &d.object_guid == id)
                .unwrap()
                .name
                .as_str()
        })
        .collect::<Vec<_>>();
    let resources = owner
        .information_register_resources
        .as_ref()
        .into_iter()
        .flatten()
        .map(|id| {
            metadata
                .descriptors()
                .iter()
                .find(|d| &d.object_guid == id)
                .unwrap()
                .name
                .clone()
        })
        .collect::<Vec<_>>();
    let filter = if names.is_empty() {
        "Новый Структура".to_string()
    } else {
        format!(
            "Новый Структура(\"{}\", {})",
            names.join(","),
            (0..names.len())
                .map(|i| (i + 7).to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let period = if change == 8 {
        "Дата(2024, 1, 2), "
    } else {
        ""
    };
    let source = format!("Возврат РегистрыСведений.ОпцииПробы.Получить({period}{filter});");
    let access = RejectRead {
        dimensions: names.into_iter().map(str::to_owned).collect(),
        resources,
        day: change == 8,
    };
    let catalog = project(catalog, &metadata).unwrap();
    let session = SessionData::new(
        Arc::new(catalog),
        Principal::new("selected-user", "Тест", vec![]),
    )
    .with_data_access(Rc::new(access));
    Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(session))
        .build()
        .exec(&source)
        .unwrap_err()
        .to_string()
}

#[test]
fn acquired_independent_fields_reach_the_current_principal_adapter() {
    let error = read_error(0);
    assert!(error.contains("projection-reader-rejected"), "{error}");
    let error = read_error(7);
    assert!(error.contains("projection-reader-rejected"), "{error}");
}

#[test]
fn missing_declarations_and_subordinate_registers_stay_unknown() {
    for (change, expected) in [
        (1, "периодичность регистра неизвестна"),
        (2, "полный состав измерений неизвестен"),
        (3, "полный состав ресурсов неизвестен"),
        (4, "периодичность регистра неизвестна"),
        (5, "периодичность регистра неизвестна"),
        (6, "период должен быть датой"),
    ] {
        let error = read_error(change);
        assert!(error.contains(expected), "change={change}: {error}");
    }
}

#[test]
fn acquired_field_order_and_day_period_reach_the_adapter() {
    for change in [8, 9] {
        let error = read_error(change);
        assert!(error.contains("projection-reader-rejected"), "{error}");
    }
}

#[test]
fn a_missing_register_field_descriptor_keeps_the_reader_incomplete() {
    for dimensions in [true, false] {
        let (catalog, metadata) = native("register-native", |descriptors| {
            let owner = descriptors.iter().find(|d| d.name == "ОпцииПробы").unwrap();
            let ids = if dimensions {
                &owner.information_register_dimensions
            } else {
                &owner.information_register_resources
            };
            let missing = ids.as_ref().unwrap()[0].clone();
            descriptors.retain(|field| field.object_guid != missing);
        });
        let owner = metadata
            .descriptors()
            .iter()
            .find(|d| d.name == "ОпцииПробы")
            .unwrap();
        assert!(if dimensions {
            owner.information_register_dimensions.is_none()
        } else {
            owner.information_register_resources.is_none()
        });
        let catalog = project(catalog, &metadata).unwrap();
        let session = SessionData::new(
            Arc::new(catalog),
            Principal::new("selected-user", "Тест", vec![]),
        );
        let error = Engine::builder()
            .register_library(bsl_orm::library())
            .build()
            .unwrap()
            .state_builder()
            .host_data(Rc::new(session))
            .build()
            .exec("Возврат РегистрыСведений.ОпцииПробы.Получить(Новый Структура);")
            .unwrap_err()
            .to_string();
        let expected = if dimensions {
            "полный состав измерений неизвестен"
        } else {
            "полный состав ресурсов неизвестен"
        };
        assert!(error.contains(expected), "{error}");
    }
}
