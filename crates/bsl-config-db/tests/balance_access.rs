//! Приобретённые поля регистра проходят проверку прав до компиляции SQL.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use bsl_orm::{
    DataAccess, DocumentAttribute, MetadataCatalog, MetadataKind, MetadataObject, Principal,
    QueryRequest, QueryResultData, SessionData,
};
use open_bsl::{CallContext, ColumnType, RtError, RtResult, TypeId, TypeRef, Value};
use open_sdbl::metadata::{
    DbNameEntry, DbNames, Guid, InfoBaseUser, LiveColumn, LiveTable, Right, UserRow,
    parse_config_descriptors, parse_role_rights, parse_schema_storage, resolve_metadata,
};
use open_sdbl::query::{
    CompileOptions, PostgresBackend, PrepareOptions, QueryCompiler, SessionParameters,
    TempTablesManager,
};
use open_sdbl_db::{AccessStore, check_user_field_access, user_decisions};

const USER_ID: &str = "11111111-1111-1111-1111-111111111111";

#[derive(Debug, Default)]
struct Capture(RefCell<Option<QueryRequest>>);

impl DataAccess for Capture {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("контроль не читает константы".into()))
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), USER_ID);
        assert_eq!(request.parameters().count(), 0);
        self.0.replace(Some(request.clone()));
        // Пустой набор нужен только для возврата из ORM; проверка БД здесь не заявляется.
        QueryResultData::new(
            ["ПолеОстатков0".into(), "ПолеОстатков1".into()],
            Vec::<Vec<Value>>::new(),
        )
        .map_err(|error| RtError::DynamicError(error.to_string()))
    }
}

#[test]
fn acquired_balance_fields_compile_with_selected_users_read_permission() {
    let owner: Guid = "c72d739a-05e3-4e9e-bcc7-3bf8baf5dc63".parse().unwrap();
    let descriptors = parse_config_descriptors(
        owner.as_str(),
        include_bytes!("fixtures/runtime-object-099/ОстаткиПорядок.deflate"),
    )
    .unwrap();
    let register = descriptors
        .iter()
        .find(|descriptor| descriptor.object_guid == owner)
        .unwrap();
    let dimensions = register.accumulation_register_dimensions.as_ref().unwrap();
    let resources = register.accumulation_register_resources.as_ref().unwrap();
    let field_name = |id: &Guid| {
        descriptors
            .iter()
            .find(|descriptor| &descriptor.object_guid == id)
            .unwrap()
            .name
            .clone()
    };
    let dimension = field_name(&dimensions[0]);
    let resource = field_name(&resources[0]);
    let attributes = |ids: &[Guid], native, qualifiers: &[&str]| {
        ids.iter()
            .map(|id| {
                DocumentAttribute::new(
                    id.as_str(),
                    field_name(id),
                    vec![ColumnType {
                        id: TypeRef::Native(native),
                        quals: qualifiers.iter().map(|value| (*value).into()).collect(),
                    }],
                )
            })
            .collect()
    };
    let catalog = MetadataCatalog::new(
        "balance-access",
        vec![MetadataObject::new(
            owner.as_str(),
            &register.name,
            MetadataKind::AccumulationRegister,
        )],
    )
    .unwrap()
    .with_accumulation_register_metadata(
        owner.as_str(),
        Some(bsl_orm::AccumulationRegisterKind::Balance),
        Some(attributes(dimensions, TypeId::String, &["12", "1"])),
        Some(attributes(resources, TypeId::Number, &["15", "2", "0"])),
    )
    .unwrap();
    let capture = Rc::new(Capture::default());
    let host = SessionData::new(Arc::new(catalog), Principal::new(USER_ID, "Тест", vec![]))
        .with_data_access(capture.clone());
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine.state_builder().host_data(Rc::new(host)).build();
    state
        .exec(&format!(
            "Возврат РегистрыНакопления.{}.Остатки(, , \"{dimension}\", \"{resource}\");",
            register.name
        ))
        .unwrap();
    let request = capture.0.borrow_mut().take().unwrap();
    let fields = dimensions.iter().chain(resources).collect::<Vec<_>>();
    let mut names = vec![
        DbNameEntry {
            guid: owner.clone(),
            alias: "AccumRg".into(),
            number: 1,
        },
        DbNameEntry {
            guid: owner,
            alias: "AccumRgT".into(),
            number: 2,
        },
    ];
    let mut columns = vec!["{\"Period\",0,{1,{\"T\",0,0,\"\",0}},\"\",0}".to_owned()];
    let mut live_columns = vec![LiveColumn {
        name: "_period".into(),
        data_type: "timestamp without time zone".into(),
    }];
    for (index, id) in fields.iter().enumerate() {
        let number = index as u32 + 10;
        names.push(DbNameEntry {
            guid: (*id).clone(),
            alias: "Fld".into(),
            number,
        });
        let tag = if index < dimensions.len() { "S" } else { "N" };
        columns.push(format!(
            "{{\"Fld{number}\",0,{{1,{{\"{tag}\",2147483657,0,\"\",0}}}},\"\",0}}"
        ));
        live_columns.push(LiveColumn {
            name: format!("_fld{number}"),
            data_type: if tag == "S" {
                "mvarchar(12)"
            } else {
                "numeric(15,2)"
            }
            .into(),
        });
    }
    for (name, tag, sql_type) in [
        ("Active", "L", "boolean"),
        ("RecordKind", "N", "numeric(1,0)"),
        ("Splitter", "N", "numeric(10,0)"),
    ] {
        columns.push(format!(
            "{{\"{name}\",0,{{1,{{\"{tag}\",0,0,\"\",0}}}},\"\",0}}"
        ));
        live_columns.push(LiveColumn {
            name: format!("_{}", name.to_ascii_lowercase()),
            data_type: sql_type.into(),
        });
    }
    // Синтетическая физическая схема проверяет связку API; запрос к БД не выполняется.
    let tables = [("AccumRg1", 1), ("AccumRgT2", 2)].map(|(name, number)| {
        format!(
            "{{\"{name}\",\"N\",{number},\"\",{{{},{}}},{{0}},{{0}},1,\"R\",{{0}},{{0}},\"\",0}}",
            columns.len(),
            columns.join(",")
        )
    });
    let schema =
        parse_schema_storage(format!("{{0,{{2,{}}}}}", tables.join(",")).as_bytes()).unwrap();
    let live = ["_accumrg1", "_accumrgt2"].map(|name| LiveTable {
        name: name.into(),
        columns: live_columns.clone(),
        indexes: vec![],
    });
    let snapshot = resolve_metadata(
        DbNames::from_entries(names),
        descriptors,
        schema,
        live.into(),
    )
    .snapshot;
    let role: Guid = "22222222-2222-2222-2222-222222222222".parse().unwrap();
    let record = format!("{{{USER_ID},\"balance-user\",\"\",{{1,{role}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
    let user = InfoBaseUser::new(
        UserRow {
            name: "balance-user",
            description: "",
            os_name: "",
            email: "",
            show_in_list: false,
            standard_authentication: true,
            administrative: false,
        },
        &encoded,
    )
    .unwrap();
    assert_eq!(user.data.id.as_str(), USER_ID);
    let rights = parse_role_rights(include_bytes!("fixtures/balance-access/read.deflate")).unwrap();
    let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
    let prepared = compiler
        .prepare_with_options(request.text(), &PrepareOptions::new().restricted())
        .unwrap();
    assert!(!prepared.field_usage().fields.is_empty());
    assert!(!prepared.restriction_request().targets.is_empty());
    assert!(
        check_user_field_access(
            &AccessStore::new(&snapshot),
            &snapshot,
            prepared.field_usage()
        )
        .is_err()
    );
    for allowed in [true, false] {
        let mut rights = rights.clone();
        rights.objects[0].rights[0].allowed = allowed;
        assert_eq!(rights.objects[0].rights[0].right, Right::Read);
        let mut access = AccessStore::new(&snapshot);
        access.set_current_user(Some(user.clone()));
        access.insert_rights(role.clone(), rights);
        let result = check_user_field_access(&access, &snapshot, prepared.field_usage());
        if !allowed {
            assert!(
                result.is_err(),
                "отказ Read должен остановить проверку полей"
            );
            continue;
        }
        result.unwrap();
        let decisions = user_decisions(
            &access,
            &snapshot,
            prepared.restriction_request(),
            &[],
            &SessionParameters::new(),
        )
        .unwrap();
        prepared
            .compile_batch(
                &snapshot,
                &CompileOptions::new().decisions(&decisions),
                &mut TempTablesManager::new(),
            )
            .unwrap();
    }
}
