use super::*;
use crate::{ObjectProtocol, TypeDescriptor, TypeRef, ValueTypeDescriptor};
use std::sync::Arc;

static OBJECT_TYPE: TypeDescriptor = TypeDescriptor::new("codec-test", "Снимок");
#[derive(Debug)]
struct Immutable {
    ty: Arc<ValueTypeDescriptor>,
    bytes: Vec<u8>,
}
impl ObjectProtocol for Immutable {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &OBJECT_TYPE
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.ty))
    }
    fn immutable_snapshot(&self) -> Option<&[u8]> {
        Some(&self.bytes)
    }
}
fn description(id: &str) -> Arc<ValueTypeDescriptor> {
    Arc::new(ValueTypeDescriptor::new(
        "codec-test",
        id.to_string(),
        "Снимок",
        ["СнимокКодека"],
    ))
}
fn shapes(ty: Arc<ValueTypeDescriptor>, codec: bool) -> RuntimeShapes {
    let mut builder = crate::RuntimeBuilder::new();
    builder.register(crate::core_library());
    let registered = if codec {
        let original = Arc::clone(&ty);
        Arc::new(
            ty.as_ref()
                .clone()
                .with_materializer(Arc::new(move |bytes| {
                    if bytes.len() != 16 {
                        return Err(RtError::DynamicError("неверный размер".into()));
                    }
                    Ok(BslValue::new_object(Immutable {
                        ty: Arc::clone(&original),
                        bytes: bytes.to_vec(),
                    }))
                })),
        )
    } else {
        ty
    };
    builder.register_value_type(registered);
    let registry = builder.build().unwrap();
    RuntimeShapes::seeded(vec![], vec![], Some(&registry))
}
#[test]
fn immutable_graph_round_trip_has_exact_budget_and_rejects_wrong_receiving_identity() {
    let ty = description("original");
    let mut rt = shapes(Arc::clone(&ty), true);
    let value = BslValue::new_object(Immutable {
        ty,
        bytes: vec![7; 16],
    });
    let values = [value];
    let cost = SerializedValueGraph::measure(&values, &rt, &GraphLimits::default()).unwrap();
    let graph =
        SerializedValueGraph::capture(&values, &rt, &GraphLimits { max_bytes: cost }).unwrap();
    assert!(
        SerializedValueGraph::capture(
            &values,
            &rt,
            &GraphLimits {
                max_bytes: cost - 1
            }
        )
        .is_err()
    );
    assert_eq!(graph.byte_size(), cost);
    let result = graph.materialize(&mut rt).unwrap();
    let BslValue::Object(object) = &result[0] else {
        panic!("объект");
    };
    let BslObject::Extension(object) = object.as_ref() else {
        panic!("расширение");
    };
    assert_eq!(object.immutable_snapshot(), Some([7; 16].as_slice()));
    assert!(
        graph
            .materialize(&mut shapes(description("other"), true))
            .is_err()
    );
    assert!(
        graph
            .materialize(&mut shapes(description("original"), false))
            .is_err()
    );
    assert!(
        graph
            .materialize(&mut RuntimeShapes::seeded(vec![], vec![], None))
            .is_err()
    );
}
#[test]
fn missing_codec_is_rejected_before_budget_and_malformed_payload_is_rejected_by_decoder() {
    let ty = description("original");
    let value = BslValue::new_object(Immutable {
        ty: Arc::clone(&ty),
        bytes: vec![7],
    });
    let without = shapes(Arc::clone(&ty), false);
    let error = SerializedValueGraph::capture(
        std::slice::from_ref(&value),
        &without,
        &GraphLimits { max_bytes: 0 },
    )
    .unwrap_err();
    assert!(!error.to_string().contains(BUDGET_EXCEEDED));
    let mut with = shapes(ty, true);
    let graph = SerializedValueGraph::capture(&[value], &with, &GraphLimits::default()).unwrap();
    assert!(graph.materialize(&mut with).is_err());
}
#[test]
fn uuid_round_trip_preserves_all_bytes_and_new_variants_do_not_enlarge_old_node_headers() {
    let mut rt = RuntimeShapes::seeded(vec![], vec![], None);
    let value = BslValue::Object(Rc::new(BslObject::Uuid([0xa5; 16])));
    let graph =
        SerializedValueGraph::capture(std::slice::from_ref(&value), &rt, &GraphLimits::default())
            .unwrap();
    assert_eq!(graph.materialize(&mut rt).unwrap(), vec![value]);
    assert_eq!(
        std::mem::size_of::<GraphNode>(),
        std::mem::size_of::<(Vec<GraphColumn>, Vec<Vec<NodeId>>)>()
    );
}
#[test]
fn registered_decoder_cannot_return_a_different_value_type() {
    let ty = description("original")
        .as_ref()
        .clone()
        .with_materializer(Arc::new(|_| Ok(BslValue::Undefined)));
    assert!(ty.materialize(&[]).is_err());
}
